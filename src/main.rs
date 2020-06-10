use dsync::{get_token, DB};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await?;
    /*let result = dsync::files::list_folder::list_folder("", &token).await?;
    dbg!(result);*/
    let data = dsync::files::download::download("/milky-way-nasa.jpg", &token).await?;
    dbg!(dsync::bytes_to_hex_string(&dsync::content_hash(&data)));

    let db = DB.lock().unwrap().open()?;
    db.put(
        None,
        "hoge".to_owned().into_bytes().as_mut_slice(),
        "fuga".to_owned().into_bytes().as_mut_slice(),
        libdb::Flags::DB_NONE,
    )
    .unwrap();

    Ok(())
}
