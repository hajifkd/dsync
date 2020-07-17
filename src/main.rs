use dsync::get_token;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await?;
    //let result = dsync::files::list_folder::list_folder("", &token).await?;
    //dbg!(result);
    let data = dsync::files::download::download("/milky-way-nasa.jpg", &token).await?;
    dbg!(&data.0);
    dbg!(dsync::bytes_to_hex_string(&dsync::content_hash(&data.1)));

    Ok(())
}
