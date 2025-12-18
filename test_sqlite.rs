use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Test 1: sqlite:filename
    println!("Test 1: sqlite:data/config.db");
    match SqlitePoolOptions::new().connect("sqlite:data/config.db").await {
        Ok(_) => println!("✅ Works!"),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 2: sqlite://filename  
    println!("\nTest 2: sqlite://data/config.db");
    match SqlitePoolOptions::new().connect("sqlite://data/config.db").await {
        Ok(_) => println!("✅ Works!"),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    // Test 3: sqlite:./data/config.db
    println!("\nTest 3: sqlite:./data/config.db");
    match SqlitePoolOptions::new().connect("sqlite:./data/config.db").await {
        Ok(_) => println!("✅ Works!"),
        Err(e) => println!("❌ Error: {}", e),
    }
    
    Ok(())
}
