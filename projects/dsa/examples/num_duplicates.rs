use std::collections::HashSet;

fn main() {
    let mut vec = vec![1, 2, 3, 4];

    println!("{}", contains_duplicates(&mut vec))
}

fn contains_duplicates(num: &mut [i32]) -> bool {
    num.sort_unstable();
    num.windows(2).any(|w| w[0] == w[1])
}

fn contains_duplicates_using_hashset(num: &mut [i32]) -> bool {
    let mut seen = HashSet::new();
    for item in num {
        if !seen.insert(item) {
            return true;
        }
    }
    false
}
