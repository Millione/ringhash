use ringhash::Consistent;

fn main() {
    let c = Consistent::new();
    c.add("cacheA");
    c.add("cacheB");
    c.add("cacheC");
    let users = vec![
        "user_mcnulty",
        "user_bunk",
        "user_omar",
        "user_bunny",
        "user_stringer",
    ];
    println!("initial state [A, B, C]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
    c.add("cacheD");
    c.add("cacheE");
    println!("with cacheD, cacheE added [A, B, C, D, E]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
    c.remove("cacheC");
    println!("with cacheC removed [A, B, D, E]");
    for u in users.iter() {
        let server = c.get(u).unwrap();
        println!("{} => {}", u, server);
    }
}
