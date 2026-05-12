# Error Handling in Nova

Nova provides a comprehensive error handling system that ensures consistent, structured error responses across your API.

## Core Concepts

### NovaError

The `NovaError` enum represents all possible errors in a Nova application:

```rust
pub enum NovaError {
    DatabaseError(String),          // Database connection/query errors
    ValidationError(String),        // Input validation failures
    AuthenticationError(String),    // Authentication failures
    AuthorizationError(String),     // Authorization failures (forbidden)
    NotFound(String),               // Resource not found
    Conflict(String),               // Conflict errors (e.g., duplicate)
    InternalError(String),          // Internal server errors
    BadRequest(String),             // Invalid input/bad requests
    Custom { status, error, message }, // Custom errors with status codes
}
```

### NovaResult

A type alias for ergonomic error handling:

```rust
pub type NovaResult<T> = Result<T, NovaError>;
```

### ApiResponse

Wraps successful responses in a consistent format:

```rust
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}
```

## Usage Examples

### Basic Error Handling

```rust
#[post("/users")]
pub async fn create_user(
    Json(payload): Json<CreateUserRequest>,
) -> NovaResult<Json<ApiResponse<User>>> {
    // Validate input
    if payload.email.is_empty() {
        return Err(NovaError::ValidationError(
            "Email cannot be empty".to_string()
        ));
    }

    // Create user...
    let user = User::new(payload);
    Ok(Json(ApiResponse::ok(user)))
}
```

Response on success (200 OK):
```json
{
  "success": true,
  "data": {
    "id": 1,
    "email": "user@example.com",
    "name": "John Doe"
  },
  "message": null
}
```

Response on validation error (400 Bad Request):
```json
{
  "error": "ValidationError",
  "message": "Email cannot be empty",
  "details": null
}
```

### Database Error Handling

```rust
#[get("/users/:id")]
pub async fn get_user(
    Extension(db): Extension<DatabaseConnection>,
    Path(id): Path<i32>,
) -> NovaResult<Json<ApiResponse<User>>> {
    let user = User::find_by_id(id)
        .one(&db)
        .await
        .map_err(|e| NovaError::DatabaseError(e.to_string()))?
        .ok_or_else(|| NovaError::NotFound(format!("User {} not found", id)))?;

    Ok(Json(ApiResponse::ok(user)))
}
```

### Validation Error

```rust
#[post("/emails/send")]
pub async fn send_email(
    Json(payload): Json<EmailRequest>,
) -> NovaResult<Json<ApiResponse<SendResponse>>> {
    // Validate email format
    if !payload.to.contains('@') {
        return Err(NovaError::ValidationError(
            "Invalid email format".to_string()
        ));
    }

    // Send email...
    Ok(Json(ApiResponse::ok(SendResponse { sent: true })))
}
```

### Authorization Error

```rust
#[delete("/posts/:id")]
pub async fn delete_post(
    Extension(user): Extension<User>,
    Path(id): Path<i32>,
) -> NovaResult<Json<ApiResponse<EmptyResponse>>> {
    let post = get_post(id).await?;

    if post.author_id != user.id {
        return Err(NovaError::AuthorizationError(
            "You can only delete your own posts".to_string()
        ));
    }

    post.delete().await?;
    Ok(Json(ApiResponse::ok(EmptyResponse)))
}
```

Response (403 Forbidden):
```json
{
  "error": "AuthorizationError",
  "message": "You can only delete your own posts",
  "details": null
}
```

### ListResponse for Collections

```rust
#[get("/posts")]
pub async fn list_posts(
    Extension(db): Extension<DatabaseConnection>,
) -> NovaResult<Json<ApiResponse<ListResponse<Post>>>> {
    let posts = Post::find().all(&db).await?;
    let list = ListResponse::with_total(posts.clone(), posts.len());
    
    Ok(Json(ApiResponse::ok_with_message(
        list,
        format!("Retrieved {} posts", posts.len()),
    )))
}
```

Response (200 OK):
```json
{
  "success": true,
  "data": {
    "items": [
      {"id": 1, "title": "Hello", "content": "..."},
      {"id": 2, "title": "World", "content": "..."}
    ],
    "count": 2,
    "total": 2
  },
  "message": "Retrieved 2 posts"
}
```

## Error Status Code Mapping

| Error Type | HTTP Status |
|---|---|
| `ValidationError` | 400 Bad Request |
| `BadRequest` | 400 Bad Request |
| `AuthenticationError` | 401 Unauthorized |
| `AuthorizationError` | 403 Forbidden |
| `NotFound` | 404 Not Found |
| `Conflict` | 409 Conflict |
| `DatabaseError` | 500 Internal Server Error |
| `InternalError` | 500 Internal Server Error |

## Automatic Conversions

Nova automatically converts common error types:

```rust
// From sea_orm errors (when database feature is enabled)
let user = User::find_by_id(1)
    .one(&db)
    .await?; // Automatically converts sea_orm::DbErr to NovaError::DatabaseError

// From serde_json errors
let value = serde_json::from_str(json_str)?; // Automatically converts to ValidationError
```

## Custom Errors

For errors not covered by the standard types:

```rust
#[post("/process")]
pub async fn process(
    Json(payload): Json<Payload>,
) -> NovaResult<Json<ApiResponse<Result>>> {
    if some_condition {
        return Err(NovaError::Custom {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            error: "ProcessingError".to_string(),
            message: "Failed to process data".to_string(),
        });
    }
    
    Ok(Json(ApiResponse::ok(Result { success: true })))
}
```

## Best Practices

1. **Use specific error types** - Choose the most appropriate `NovaError` variant
2. **Provide meaningful messages** - Help API consumers understand what went wrong
3. **Use NovaResult** - Always return `NovaResult<T>` from handlers
4. **Leverage auto-conversion** - Let Nova convert database and serialization errors
5. **Validate early** - Check input validity before processing
6. **Use ApiResponse consistently** - Wrap all successful responses
7. **Handle database errors** - Always use `?` operator or `.map_err()` for DB operations

## Migration from Raw Responses

**Before:**
```rust
#[get("/users")]
pub async fn get_users() -> String {
    "Error: something went wrong".to_string()
}
```

**After:**
```rust
#[get("/users")]
pub async fn get_users() -> NovaResult<Json<ApiResponse<Vec<User>>>> {
    let users = fetch_users().await?;
    Ok(Json(ApiResponse::ok(users)))
}
```
