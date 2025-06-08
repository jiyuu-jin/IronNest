use {
    clap::{Parser, Subcommand},
    reqwest::Url,
    std::path::PathBuf,
};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Options {
    #[arg(long, env)]
    pub server_url: Option<Url>,

    #[command(subcommand)]
    pub operation: Operation,
}

#[derive(Subcommand, Debug)]
#[command()]
pub enum Operation {
    // #[command(long)]
    // DownloadFile {
    //     cid: String,
    //     path: PathBuf,
    // },
    #[command()]
    UploadFile {
        file_path: PathBuf,

        #[arg(long)]
        mish_state_name: String,

        #[arg(long)]
        path: String,
    },
    // ReadMishState {
    //     name: String,
    // },

    // WriteMishState {
    //     name: String,
    //     path: PathBuf,
    // },
}
