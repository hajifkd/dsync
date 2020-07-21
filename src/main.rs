use dsync::{db, get_token};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let token = get_token().await?;
    //let result = dsync::files::list_folder::list_folder("", &token).await?;
    //dbg!(result);
    let data = dsync::files::download::download("/milky-way-nasa.jpg", &token).await?;
    dbg!(&data.0);
    dbg!(dsync::bytes_to_hex_string(&dsync::content_hash(&data.1)));
    let mut c = db::connect()?;
    db::upsert_files(
        &mut c,
        &[
            db::FileData::new("hoge".to_owned(), vec![1, 2, 3, 4, 5]),
            db::FileData::new("fuga".to_owned(), vec![1, 2, 3, 4, 5]),
        ],
    )?;

    db::upsert_files(
        &mut c,
        &[
            db::FileData::new("hoge".to_owned(), vec![1, 2, 3, 4, 5, 6]),
            db::FileData::new("piyo".to_owned(), vec![1, 2, 3, 4, 5, 6]),
        ],
    )?;

    db::clear_files_to_update(&mut c, &["a"])?;

    Ok(())
}
