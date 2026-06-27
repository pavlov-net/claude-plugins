fn total(items: &[Item]) -> u64 {
    // Initialize the sum to zero.
    let mut sum = 0;
    // Loop over each item in the list.
    for item in items {
        // Add the item's price to the running sum.
        sum += item.price;
    }
    // Return the sum.
    sum
}
