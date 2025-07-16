/**
 * Service Worker for Code Mesh PWA
 * Provides offline support and caching for WASM modules
 */

const CACHE_NAME = 'code-mesh-v1';
const STATIC_CACHE_NAME = 'code-mesh-static-v1';
const WASM_CACHE_NAME = 'code-mesh-wasm-v1';

// Files to cache for offline use
const STATIC_FILES = [
  '/',
  '/index.html',
  '/manifest.json',
  '/dist/browser.js',
  '/dist/index.js',
  '/wasm/web/code_mesh_wasm.js',
  '/wasm/web/code_mesh_wasm_bg.wasm'
];

// Install event - cache static files
self.addEventListener('install', (event) => {
  console.log('Service Worker: Installing...');
  
  event.waitUntil(
    Promise.all([
      // Cache static files
      caches.open(STATIC_CACHE_NAME).then((cache) => {
        console.log('Service Worker: Caching static files');
        return cache.addAll(STATIC_FILES);
      }),
      
      // Cache WASM files separately for better control
      caches.open(WASM_CACHE_NAME).then((cache) => {
        console.log('Service Worker: Caching WASM files');
        const wasmFiles = STATIC_FILES.filter(file => file.includes('wasm'));
        return cache.addAll(wasmFiles);
      })
    ]).then(() => {
      console.log('Service Worker: Installation complete');
      // Force activation
      return self.skipWaiting();
    })
  );
});

// Activate event - clean up old caches
self.addEventListener('activate', (event) => {
  console.log('Service Worker: Activating...');
  
  event.waitUntil(
    caches.keys().then((cacheNames) => {
      return Promise.all(
        cacheNames.map((cacheName) => {
          // Delete old caches
          if (cacheName !== CACHE_NAME && 
              cacheName !== STATIC_CACHE_NAME && 
              cacheName !== WASM_CACHE_NAME) {
            console.log('Service Worker: Deleting old cache:', cacheName);
            return caches.delete(cacheName);
          }
        })
      );
    }).then(() => {
      console.log('Service Worker: Activation complete');
      // Take control of all pages
      return self.clients.claim();
    })
  );
});

// Fetch event - serve from cache when offline
self.addEventListener('fetch', (event) => {
  const { request } = event;
  const url = new URL(request.url);
  
  // Skip non-HTTP requests
  if (!url.protocol.startsWith('http')) {
    return;
  }
  
  // Handle different types of requests
  if (isWasmRequest(request)) {
    event.respondWith(handleWasmRequest(request));
  } else if (isStaticRequest(request)) {
    event.respondWith(handleStaticRequest(request));
  } else if (isApiRequest(request)) {
    event.respondWith(handleApiRequest(request));
  } else {
    event.respondWith(handleGenericRequest(request));
  }
});

// Check if request is for WASM files
function isWasmRequest(request) {
  return request.url.includes('.wasm') || 
         request.url.includes('wasm/') ||
         request.url.includes('code_mesh_wasm');
}

// Check if request is for static files
function isStaticRequest(request) {
  const url = new URL(request.url);
  return STATIC_FILES.some(file => url.pathname.endsWith(file)) ||
         request.destination === 'script' ||
         request.destination === 'style' ||
         request.destination === 'document';
}

// Check if request is for API calls
function isApiRequest(request) {
  return request.url.includes('/api/') ||
         request.url.includes('api.anthropic.com') ||
         request.url.includes('api.openai.com');
}

// Handle WASM requests with aggressive caching
async function handleWasmRequest(request) {
  const cache = await caches.open(WASM_CACHE_NAME);
  const cached = await cache.match(request);
  
  if (cached) {
    console.log('Service Worker: Serving WASM from cache:', request.url);
    return cached;
  }
  
  try {
    console.log('Service Worker: Fetching WASM:', request.url);
    const response = await fetch(request);
    
    if (response.ok) {
      // Clone response for caching
      const responseClone = response.clone();
      await cache.put(request, responseClone);
      console.log('Service Worker: WASM cached:', request.url);
    }
    
    return response;
  } catch (error) {
    console.error('Service Worker: WASM fetch failed:', error);
    throw error;
  }
}

// Handle static file requests
async function handleStaticRequest(request) {
  const cache = await caches.open(STATIC_CACHE_NAME);
  const cached = await cache.match(request);
  
  if (cached) {
    console.log('Service Worker: Serving static from cache:', request.url);
    return cached;
  }
  
  try {
    console.log('Service Worker: Fetching static:', request.url);
    const response = await fetch(request);
    
    if (response.ok) {
      const responseClone = response.clone();
      await cache.put(request, responseClone);
      console.log('Service Worker: Static file cached:', request.url);
    }
    
    return response;
  } catch (error) {
    console.error('Service Worker: Static fetch failed:', error);
    
    // Try to serve offline fallback
    if (request.destination === 'document') {
      const offlinePage = await cache.match('/offline.html');
      if (offlinePage) {
        return offlinePage;
      }
    }
    
    throw error;
  }
}

// Handle API requests (no caching for security)
async function handleApiRequest(request) {
  try {
    console.log('Service Worker: API request:', request.url);
    return await fetch(request);
  } catch (error) {
    console.error('Service Worker: API request failed:', error);
    
    // Return error response for API failures
    return new Response(
      JSON.stringify({
        error: 'Network error',
        message: 'API request failed - check your internet connection',
        offline: true
      }),
      {
        status: 503,
        statusText: 'Service Unavailable',
        headers: {
          'Content-Type': 'application/json',
          'X-Served-By': 'ServiceWorker'
        }
      }
    );
  }
}

// Handle generic requests
async function handleGenericRequest(request) {
  try {
    return await fetch(request);
  } catch (error) {
    console.error('Service Worker: Generic fetch failed:', error);
    throw error;
  }
}

// Message handling for communication with main thread
self.addEventListener('message', (event) => {
  const { type, payload } = event.data;
  
  switch (type) {
    case 'SKIP_WAITING':
      self.skipWaiting();
      break;
      
    case 'CACHE_WASM':
      cacheWasmFiles(payload.files);
      break;
      
    case 'CLEAR_CACHE':
      clearAllCaches();
      break;
      
    case 'GET_CACHE_SIZE':
      getCacheSize().then(size => {
        event.ports[0].postMessage({ type: 'CACHE_SIZE', size });
      });
      break;
      
    default:
      console.warn('Service Worker: Unknown message type:', type);
  }
});

// Cache additional WASM files
async function cacheWasmFiles(files) {
  const cache = await caches.open(WASM_CACHE_NAME);
  
  for (const file of files) {
    try {
      const response = await fetch(file);
      if (response.ok) {
        await cache.put(file, response);
        console.log('Service Worker: Cached WASM file:', file);
      }
    } catch (error) {
      console.error('Service Worker: Failed to cache WASM file:', file, error);
    }
  }
}

// Clear all caches
async function clearAllCaches() {
  const cacheNames = await caches.keys();
  
  await Promise.all(
    cacheNames.map(cacheName => {
      console.log('Service Worker: Clearing cache:', cacheName);
      return caches.delete(cacheName);
    })
  );
  
  console.log('Service Worker: All caches cleared');
}

// Get total cache size
async function getCacheSize() {
  const cacheNames = await caches.keys();
  let totalSize = 0;
  
  for (const cacheName of cacheNames) {
    const cache = await caches.open(cacheName);
    const keys = await cache.keys();
    
    for (const request of keys) {
      const response = await cache.match(request);
      if (response) {
        const blob = await response.blob();
        totalSize += blob.size;
      }
    }
  }
  
  return totalSize;
}

// Background sync for offline actions
self.addEventListener('sync', (event) => {
  console.log('Service Worker: Background sync:', event.tag);
  
  if (event.tag === 'sync-sessions') {
    event.waitUntil(syncSessions());
  }
});

// Sync sessions when back online
async function syncSessions() {
  try {
    // Get pending sessions from IndexedDB
    const db = await openDB();
    const transaction = db.transaction(['pending_sync'], 'readonly');
    const store = transaction.objectStore('pending_sync');
    const pendingSessions = await store.getAll();
    
    // Upload pending sessions
    for (const session of pendingSessions) {
      try {
        await uploadSession(session);
        await deletePendingSession(session.id);
        console.log('Service Worker: Synced session:', session.id);
      } catch (error) {
        console.error('Service Worker: Failed to sync session:', session.id, error);
      }
    }
  } catch (error) {
    console.error('Service Worker: Sync failed:', error);
  }
}

// Helper functions for IndexedDB operations
async function openDB() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open('code_mesh_sync', 1);
    
    request.onerror = () => reject(request.error);
    request.onsuccess = () => resolve(request.result);
    
    request.onupgradeneeded = () => {
      const db = request.result;
      if (!db.objectStoreNames.contains('pending_sync')) {
        db.createObjectStore('pending_sync', { keyPath: 'id' });
      }
    };
  });
}

async function uploadSession(session) {
  const response = await fetch('/api/sessions', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json'
    },
    body: JSON.stringify(session)
  });
  
  if (!response.ok) {
    throw new Error(`Upload failed: ${response.status}`);
  }
  
  return response.json();
}

async function deletePendingSession(sessionId) {
  const db = await openDB();
  const transaction = db.transaction(['pending_sync'], 'readwrite');
  const store = transaction.objectStore('pending_sync');
  await store.delete(sessionId);
}