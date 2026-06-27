fn poll_vendor(client: &Client) -> Result<Status, Error> {
    // We deliberately sleep for a hundred milliseconds here before making the
    // next call, because the vendor's API has been observed to return HTTP 429
    // "too many requests" errors whenever two calls are made less than 100ms
    // apart (see issue SUPPORT-3391), and so this sleep is unfortunately
    // necessary and really must not be removed even though it looks pointless.
    std::thread::sleep(Duration::from_millis(100));
    client.get_status()
}
