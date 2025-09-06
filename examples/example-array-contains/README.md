# Array Contains Example

This example demonstrates the `assert_array_contains` method, which works similarly to `assert_json_contains` but specifically for array responses with partial matching capabilities.

## Features Demonstrated

- **Partial Object Matching**: Verify that objects with specific fields are present in an array, without requiring exact matches
- **Exact Object Matching**: Verify complete objects are present in the array
- **Simple Value Matching**: Check for primitive values in arrays
- **Single Element Matching**: Verify individual elements exist
- **Empty Array Handling**: Empty arrays are always valid subsets
- **Multiple Element Matching**: Check for multiple items at once

## Usage Examples

```rust
// Partial object matching - only checks specified fields
server.get("/users")
    .await
    .assert_array_contains(&json!([
        {"name": "Alice", "role": "admin"},
        {"name": "Bob"}  // Only checks name, ignores other fields
    ]));

// Exact object matching
server.get("/products")
    .await
    .assert_array_contains(&json!([
        {
            "id": 101,
            "name": "Laptop",
            "price": 999.99,
            "category": "Electronics"
        }
    ]));

// Simple value matching
server.get("/tags")
    .await
    .assert_array_contains(&json!(["rust", "testing"]));
```

## How It Differs from `assert_json_contains`

- `assert_json_contains`: Works on JSON objects, checks if specified fields are contained within the object
- `assert_array_contains`: Works on JSON arrays, checks if specified elements (with partial matching) are contained within the array

## Run the Example

```bash
cargo run --example example-array-contains
```

This will demonstrate all the different ways `assert_array_contains` can be used for testing array responses with partial matching.
