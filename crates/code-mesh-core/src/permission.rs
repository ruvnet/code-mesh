//! Permission system for Code Mesh Core
//!
//! This module provides a comprehensive permission system for controlling
//! access to tools, resources, and operations within the Code Mesh ecosystem.

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Permission levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionLevel {
    /// No access allowed
    None = 0,
    
    /// Read-only access
    Read = 1,
    
    /// Restricted access with limitations
    Restricted = 2,
    
    /// Standard access for normal operations
    Standard = 3,
    
    /// Elevated access for advanced operations
    Elevated = 4,
    
    /// Full administrative access
    Admin = 5,
}

impl Default for PermissionLevel {
    fn default() -> Self {
        PermissionLevel::Restricted
    }
}

impl std::str::FromStr for PermissionLevel {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "none" => Ok(PermissionLevel::None),
            "read" => Ok(PermissionLevel::Read),
            "restricted" => Ok(PermissionLevel::Restricted),
            "standard" => Ok(PermissionLevel::Standard),
            "elevated" => Ok(PermissionLevel::Elevated),
            "admin" => Ok(PermissionLevel::Admin),
            _ => Err(Error::Other(anyhow::anyhow!("Invalid permission level: {}", s))),
        }
    }
}

impl std::fmt::Display for PermissionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PermissionLevel::None => write!(f, "none"),
            PermissionLevel::Read => write!(f, "read"),
            PermissionLevel::Restricted => write!(f, "restricted"),
            PermissionLevel::Standard => write!(f, "standard"),
            PermissionLevel::Elevated => write!(f, "elevated"),
            PermissionLevel::Admin => write!(f, "admin"),
        }
    }
}

/// Permission for a specific resource or operation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    /// The resource being accessed
    pub resource: String,
    
    /// The operation being performed
    pub operation: String,
    
    /// Required permission level
    pub level: PermissionLevel,
    
    /// Additional constraints
    pub constraints: Vec<PermissionConstraint>,
}

/// Constraints that can be applied to permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PermissionConstraint {
    /// Path-based constraints
    PathConstraint {
        /// Allowed paths or patterns
        allowed: Vec<String>,
        /// Denied paths or patterns
        denied: Vec<String>,
    },
    
    /// Size-based constraints
    SizeConstraint {
        /// Maximum size in bytes
        max_size: u64,
    },
    
    /// Time-based constraints
    TimeConstraint {
        /// Maximum execution time in seconds
        max_duration: u64,
    },
    
    /// Network-based constraints
    NetworkConstraint {
        /// Whether network access is allowed
        allowed: bool,
        /// Allowed hosts/domains
        allowed_hosts: Vec<String>,
        /// Denied hosts/domains
        denied_hosts: Vec<String>,
    },
    
    /// Resource-based constraints
    ResourceConstraint {
        /// Maximum memory usage in MB
        max_memory_mb: Option<u64>,
        /// Maximum CPU usage percentage
        max_cpu_percent: Option<u32>,
    },
    
    /// Custom constraints
    CustomConstraint {
        /// Constraint name
        name: String,
        /// Constraint parameters
        params: HashMap<String, serde_json::Value>,
    },
}

/// Context for permission evaluation
#[derive(Debug, Clone)]
pub struct PermissionContext {
    /// User or session identifier
    pub user_id: String,
    
    /// Session identifier
    pub session_id: String,
    
    /// Current working directory
    pub working_dir: PathBuf,
    
    /// Requested resource
    pub resource: String,
    
    /// Requested operation
    pub operation: String,
    
    /// Additional context parameters
    pub params: HashMap<String, serde_json::Value>,
    
    /// Current permission level
    pub current_level: PermissionLevel,
}

impl PermissionContext {
    /// Create a new permission context
    pub fn new(
        user_id: String,
        session_id: String,
        working_dir: PathBuf,
        resource: String,
        operation: String,
    ) -> Self {
        Self {
            user_id,
            session_id,
            working_dir,
            resource,
            operation,
            params: HashMap::new(),
            current_level: PermissionLevel::default(),
        }
    }

    /// Add a parameter to the context
    pub fn with_param<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<serde_json::Value>,
    {
        self.params.insert(key.into(), value.into());
        self
    }

    /// Set the current permission level
    pub fn with_level(mut self, level: PermissionLevel) -> Self {
        self.current_level = level;
        self
    }
}

/// Permission manager for evaluating and enforcing permissions
pub struct PermissionManager {
    /// Default permission level
    default_level: PermissionLevel,
    
    /// Resource-specific permissions
    resource_permissions: HashMap<String, HashMap<String, Permission>>,
    
    /// User-specific permission overrides
    user_permissions: HashMap<String, HashMap<String, PermissionLevel>>,
    
    /// Global permission constraints
    global_constraints: Vec<PermissionConstraint>,
}

impl PermissionManager {
    /// Create a new permission manager
    pub fn new(default_level: PermissionLevel) -> Self {
        Self {
            default_level,
            resource_permissions: HashMap::new(),
            user_permissions: HashMap::new(),
            global_constraints: Vec::new(),
        }
    }

    /// Add a permission rule for a resource and operation
    pub fn add_permission(
        &mut self,
        resource: String,
        operation: String,
        permission: Permission,
    ) {
        self.resource_permissions
            .entry(resource)
            .or_insert_with(HashMap::new)
            .insert(operation, permission);
    }

    /// Set user-specific permission level for a resource
    pub fn set_user_permission(
        &mut self,
        user_id: String,
        resource: String,
        level: PermissionLevel,
    ) {
        self.user_permissions
            .entry(user_id)
            .or_insert_with(HashMap::new)
            .insert(resource, level);
    }

    /// Add a global constraint
    pub fn add_global_constraint(&mut self, constraint: PermissionConstraint) {
        self.global_constraints.push(constraint);
    }

    /// Check if an operation is permitted
    pub fn check_permission(&self, context: &PermissionContext) -> Result<bool> {
        // Get the required permission level
        let required_level = self.get_required_level(context);
        
        // Get the user's permission level
        let user_level = self.get_user_level(context);
        
        // Check if user has sufficient permission level
        if user_level < required_level {
            return Ok(false);
        }

        // Check constraints
        self.check_constraints(context)?;

        Ok(true)
    }

    /// Enforce permission (returns error if not permitted)
    pub fn enforce_permission(&self, context: &PermissionContext) -> Result<()> {
        if !self.check_permission(context)? {
            return Err(Error::Other(anyhow::anyhow!(
                "Permission denied for operation '{}' on resource '{}'",
                context.operation,
                context.resource
            )));
        }
        Ok(())
    }

    /// Get the required permission level for a context
    fn get_required_level(&self, context: &PermissionContext) -> PermissionLevel {
        if let Some(resource_perms) = self.resource_permissions.get(&context.resource) {
            if let Some(permission) = resource_perms.get(&context.operation) {
                return permission.level;
            }
        }
        
        // Check for wildcard operations
        if let Some(resource_perms) = self.resource_permissions.get(&context.resource) {
            if let Some(permission) = resource_perms.get("*") {
                return permission.level;
            }
        }
        
        // Check for wildcard resources
        if let Some(resource_perms) = self.resource_permissions.get("*") {
            if let Some(permission) = resource_perms.get(&context.operation) {
                return permission.level;
            }
            if let Some(permission) = resource_perms.get("*") {
                return permission.level;
            }
        }

        self.default_level
    }

    /// Get the user's permission level for a context
    fn get_user_level(&self, context: &PermissionContext) -> PermissionLevel {
        // Check user-specific permissions
        if let Some(user_perms) = self.user_permissions.get(&context.user_id) {
            if let Some(&level) = user_perms.get(&context.resource) {
                return level;
            }
            if let Some(&level) = user_perms.get("*") {
                return level;
            }
        }

        context.current_level
    }

    /// Check all applicable constraints
    fn check_constraints(&self, context: &PermissionContext) -> Result<()> {
        // Check global constraints
        for constraint in &self.global_constraints {
            self.check_constraint(constraint, context)?;
        }

        // Check resource-specific constraints
        if let Some(resource_perms) = self.resource_permissions.get(&context.resource) {
            if let Some(permission) = resource_perms.get(&context.operation) {
                for constraint in &permission.constraints {
                    self.check_constraint(constraint, context)?;
                }
            }
        }

        Ok(())
    }

    /// Check a specific constraint
    fn check_constraint(
        &self,
        constraint: &PermissionConstraint,
        context: &PermissionContext,
    ) -> Result<()> {
        match constraint {
            PermissionConstraint::PathConstraint { allowed, denied } => {
                self.check_path_constraint(allowed, denied, context)?;
            }
            PermissionConstraint::SizeConstraint { max_size } => {
                self.check_size_constraint(*max_size, context)?;
            }
            PermissionConstraint::TimeConstraint { max_duration } => {
                self.check_time_constraint(*max_duration, context)?;
            }
            PermissionConstraint::NetworkConstraint {
                allowed,
                allowed_hosts,
                denied_hosts,
            } => {
                self.check_network_constraint(*allowed, allowed_hosts, denied_hosts, context)?;
            }
            PermissionConstraint::ResourceConstraint {
                max_memory_mb,
                max_cpu_percent,
            } => {
                self.check_resource_constraint(*max_memory_mb, *max_cpu_percent, context)?;
            }
            PermissionConstraint::CustomConstraint { name, params } => {
                self.check_custom_constraint(name, params, context)?;
            }
        }
        Ok(())
    }

    /// Check path-based constraints
    fn check_path_constraint(
        &self,
        allowed: &[String],
        denied: &[String],
        context: &PermissionContext,
    ) -> Result<()> {
        // Get the target path from context
        let target_path = if let Some(path_value) = context.params.get("path") {
            PathBuf::from(path_value.as_str().unwrap_or(""))
        } else {
            return Ok(()); // No path to check
        };

        // Make path absolute relative to working directory
        let abs_path = if target_path.is_absolute() {
            target_path
        } else {
            context.working_dir.join(target_path)
        };

        // Check denied patterns first
        for pattern in denied {
            if self.path_matches_pattern(&abs_path, pattern)? {
                return Err(Error::Other(anyhow::anyhow!(
                    "Path {} is denied by pattern {}",
                    abs_path.display(),
                    pattern
                )));
            }
        }

        // If allowed patterns are specified, check them
        if !allowed.is_empty() {
            let mut path_allowed = false;
            for pattern in allowed {
                if self.path_matches_pattern(&abs_path, pattern)? {
                    path_allowed = true;
                    break;
                }
            }
            if !path_allowed {
                return Err(Error::Other(anyhow::anyhow!(
                    "Path {} is not allowed by any pattern",
                    abs_path.display()
                )));
            }
        }

        Ok(())
    }

    /// Check if a path matches a pattern (supports glob patterns)
    fn path_matches_pattern(&self, path: &Path, pattern: &str) -> Result<bool> {
        // Simple glob pattern matching
        let pattern = glob::Pattern::new(pattern)
            .map_err(|e| Error::Other(anyhow::anyhow!("Invalid glob pattern {}: {}", pattern, e)))?;
        
        Ok(pattern.matches_path(path))
    }

    /// Check size-based constraints
    fn check_size_constraint(&self, max_size: u64, context: &PermissionContext) -> Result<()> {
        if let Some(size_value) = context.params.get("size") {
            if let Some(size) = size_value.as_u64() {
                if size > max_size {
                    return Err(Error::Other(anyhow::anyhow!(
                        "Size {} exceeds maximum allowed size {}",
                        size,
                        max_size
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check time-based constraints
    fn check_time_constraint(&self, max_duration: u64, context: &PermissionContext) -> Result<()> {
        if let Some(duration_value) = context.params.get("duration") {
            if let Some(duration) = duration_value.as_u64() {
                if duration > max_duration {
                    return Err(Error::Other(anyhow::anyhow!(
                        "Duration {} exceeds maximum allowed duration {}",
                        duration,
                        max_duration
                    )));
                }
            }
        }
        Ok(())
    }

    /// Check network-based constraints
    fn check_network_constraint(
        &self,
        allowed: bool,
        allowed_hosts: &[String],
        denied_hosts: &[String],
        context: &PermissionContext,
    ) -> Result<()> {
        if !allowed {
            if context.params.contains_key("network") {
                return Err(Error::Other(anyhow::anyhow!("Network access is not allowed")));
            }
        }

        if let Some(host_value) = context.params.get("host") {
            if let Some(host) = host_value.as_str() {
                // Check denied hosts first
                for denied_host in denied_hosts {
                    if host.contains(denied_host) {
                        return Err(Error::Other(anyhow::anyhow!(
                            "Host {} is denied",
                            host
                        )));
                    }
                }

                // Check allowed hosts if specified
                if !allowed_hosts.is_empty() {
                    let mut host_allowed = false;
                    for allowed_host in allowed_hosts {
                        if host.contains(allowed_host) {
                            host_allowed = true;
                            break;
                        }
                    }
                    if !host_allowed {
                        return Err(Error::Other(anyhow::anyhow!(
                            "Host {} is not in allowed hosts list",
                            host
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check resource-based constraints
    fn check_resource_constraint(
        &self,
        max_memory_mb: Option<u64>,
        max_cpu_percent: Option<u32>,
        context: &PermissionContext,
    ) -> Result<()> {
        if let Some(max_mem) = max_memory_mb {
            if let Some(memory_value) = context.params.get("memory_mb") {
                if let Some(memory) = memory_value.as_u64() {
                    if memory > max_mem {
                        return Err(Error::Other(anyhow::anyhow!(
                            "Memory usage {} MB exceeds maximum {}MB",
                            memory,
                            max_mem
                        )));
                    }
                }
            }
        }

        if let Some(max_cpu) = max_cpu_percent {
            if let Some(cpu_value) = context.params.get("cpu_percent") {
                if let Some(cpu) = cpu_value.as_u64() {
                    if cpu > max_cpu as u64 {
                        return Err(Error::Other(anyhow::anyhow!(
                            "CPU usage {}% exceeds maximum {}%",
                            cpu,
                            max_cpu
                        )));
                    }
                }
            }
        }

        Ok(())
    }

    /// Check custom constraints (extensible)
    fn check_custom_constraint(
        &self,
        _name: &str,
        _params: &HashMap<String, serde_json::Value>,
        _context: &PermissionContext,
    ) -> Result<()> {
        // Custom constraint checking would be implemented here
        // This allows for extensible constraint types
        Ok(())
    }

    /// Create a permission manager with common defaults
    pub fn with_defaults() -> Self {
        let mut manager = Self::new(PermissionLevel::Restricted);

        // Add common permission rules
        manager.add_permission(
            "file".to_string(),
            "read".to_string(),
            Permission {
                resource: "file".to_string(),
                operation: "read".to_string(),
                level: PermissionLevel::Read,
                constraints: vec![],
            },
        );

        manager.add_permission(
            "file".to_string(),
            "write".to_string(),
            Permission {
                resource: "file".to_string(),
                operation: "write".to_string(),
                level: PermissionLevel::Standard,
                constraints: vec![
                    PermissionConstraint::SizeConstraint { max_size: 10 * 1024 * 1024 }, // 10MB
                ],
            },
        );

        manager.add_permission(
            "bash".to_string(),
            "execute".to_string(),
            Permission {
                resource: "bash".to_string(),
                operation: "execute".to_string(),
                level: PermissionLevel::Elevated,
                constraints: vec![
                    PermissionConstraint::TimeConstraint { max_duration: 300 }, // 5 minutes
                    PermissionConstraint::NetworkConstraint {
                        allowed: false,
                        allowed_hosts: vec![],
                        denied_hosts: vec![],
                    },
                ],
            },
        );

        manager.add_permission(
            "web".to_string(),
            "fetch".to_string(),
            Permission {
                resource: "web".to_string(),
                operation: "fetch".to_string(),
                level: PermissionLevel::Standard,
                constraints: vec![
                    PermissionConstraint::NetworkConstraint {
                        allowed: true,
                        allowed_hosts: vec![],
                        denied_hosts: vec!["localhost".to_string(), "127.0.0.1".to_string()],
                    },
                ],
            },
        );

        manager
    }
}

impl Default for PermissionManager {
    fn default() -> Self {
        Self::with_defaults()
    }
}