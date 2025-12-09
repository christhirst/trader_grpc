pub fn random_action() {
    let mut rng = rand::thread_rng();
    match rng.gen_bool(0.5) {
        true => println!("Action: Buy"),
        false => println!("Action: Sell"),
    }
}

#[cfg(test)]
mod tests {}
use super::*;

#[test]
fn test_random_action() {
    random_action();
}
