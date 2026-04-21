use git2::Repository;

fn main() -> anyhow::Result<()> {
    let repo = Repository::open(".")?;
    let head = repo.head()?;
    println!("HEAD: {:?}", head.shorthand());
    Ok(())
}
