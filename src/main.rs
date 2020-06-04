use dsync::get_token;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await?;
    let result = dsync::files::list_folder::list_folder("", &token).await?;
    dbg!(result);

    Ok(())
}
