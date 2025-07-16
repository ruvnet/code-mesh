//! Auth command implementation

use anyhow::Result;

pub async fn login() -> Result<()> {
    println!("Auth login not yet implemented");
    Ok(())
}

pub async fn logout(_provider: &str) -> Result<()> {
    println!("Auth logout not yet implemented");
    Ok(())
}

pub async fn list() -> Result<()> {
    println!("Auth list not yet implemented");
    Ok(())
}