use axum::{
    extract::Json,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use axum_test::TestServer;

#[derive(Serialize, Deserialize, Debug)]
struct User {
    id: u32,
    name: String,
    email: String,
    role: String,
    created_at: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Product {
    id: u32,
    name: String,
    price: f64,
    category: String,
}

// Route handler for users
async fn get_users() -> Json<Vec<User>> {
    Json(vec![
        User {
            id: 1,
            name: "Alice Johnson".to_string(),
            email: "alice@example.com".to_string(),
            role: "admin".to_string(),
            created_at: "2024-01-15T10:30:00Z".to_string(),
        },
        User {
            id: 2,
            name: "Bob Smith".to_string(),
            email: "bob@example.com".to_string(),
            role: "user".to_string(),
            created_at: "2024-02-20T14:15:00Z".to_string(),
        },
        User {
            id: 3,
            name: "Charlie Brown".to_string(),
            email: "charlie@example.com".to_string(),
            role: "moderator".to_string(),
            created_at: "2024-03-10T09:45:00Z".to_string(),
        },
    ])
}

// Route handler for products
async fn get_products() -> Json<Vec<Product>> {
    Json(vec![
        Product {
            id: 101,
            name: "Laptop".to_string(),
            price: 999.99,
            category: "Electronics".to_string(),
        },
        Product {
            id: 102,
            name: "Book".to_string(),
            price: 29.99,
            category: "Education".to_string(),
        },
        Product {
            id: 103,
            name: "Coffee Mug".to_string(),
            price: 12.50,
            category: "Home".to_string(),
        },
    ])
}

// Route handler for simple string array
async fn get_tags() -> Json<Vec<String>> {
    Json(vec![
        "rust".to_string(),
        "web".to_string(),
        "api".to_string(),
        "testing".to_string(),
        "axum".to_string(),
    ])
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create the application routes
    let app = Router::new()
        .route("/users", get(get_users))
        .route("/products", get(get_products))
        .route("/tags", get(get_tags));

    // Create a test server
    let server = TestServer::new(app)?;

    println!("🧪 Testing assert_array_contains functionality");
    println!("===============================================");

    // Example 1: Test partial object matching for users
    println!("\n1. Testing partial object matching for users:");
    server
        .get("/users")
        .await
        .assert_array_contains(&json!([
            {
                "name": "Alice Johnson",
                "role": "admin"
            },
            {
                "name": "Bob Smith"
            }
        ]));
    println!("   ✅ Successfully verified Alice (admin) and Bob are in the users array");

    // Example 2: Test exact object matching
    println!("\n2. Testing exact object matching:");
    server
        .get("/products")
        .await
        .assert_array_contains(&json!([
            {
                "id": 101,
                "name": "Laptop",
                "price": 999.99,
                "category": "Electronics"
            }
        ]));
    println!("   ✅ Successfully verified the exact Laptop product is in the array");

    // Example 3: Test simple value matching in string array
    println!("\n3. Testing simple value matching in string array:");
    server
        .get("/tags")
        .await
        .assert_array_contains(&json!(["rust", "testing"]));
    println!("   ✅ Successfully verified 'rust' and 'testing' tags are present");

    // Example 4: Test single element matching
    println!("\n4. Testing single element matching:");
    server
        .get("/users")
        .await
        .assert_array_contains(&json!([
            {
                "role": "moderator",
                "name": "Charlie Brown"
            }
        ]));
    println!("   ✅ Successfully verified Charlie Brown (moderator) is in the users array");

    // Example 5: Test empty array (should always pass)
    println!("\n5. Testing empty array (should always pass):");
    server
        .get("/users")
        .await
        .assert_array_contains(&json!([]));
    println!("   ✅ Successfully verified empty array subset");

    // Example 6: Test multiple products with partial matching
    println!("\n6. Testing multiple products with partial matching:");
    server
        .get("/products")
        .await
        .assert_array_contains(&json!([
            {
                "category": "Electronics"
            },
            {
                "name": "Book",
                "category": "Education"
            }
        ]));
    println!("   ✅ Successfully verified Electronics category and Book product");

    println!("\n🎉 All assert_array_contains tests passed!");
    println!("This demonstrates how assert_array_contains works similarly to");
    println!("assert_json_contains but specifically for array responses with");
    println!("partial matching capabilities.");

    Ok(())
}
