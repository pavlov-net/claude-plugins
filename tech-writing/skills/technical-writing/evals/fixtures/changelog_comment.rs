// We used to fetch users one at a time, but we switched to batching them in
// this PR for performance. The old per-user loop has been removed now.
fn fetch_users(ids: &[Id]) -> Vec<User> {
    batch_fetch(ids)
}
