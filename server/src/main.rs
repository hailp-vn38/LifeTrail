use clap::{Args, Parser, Subcommand};
use lifetrail_server::{
    app::{self, AppState},
    auth::generate_device_token,
    config::Config,
    db,
};
use serde::Serialize;
use std::process::ExitCode;
use uuid::Uuid;

#[derive(Parser)]
#[command(
    name = "lifetrail-server",
    about = "LifeTrail local server and provisioning CLI"
)]
struct Cli {
    #[arg(long, env = "LT_DATABASE_URL", global = true)]
    database_url: Option<String>,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Serve,
    ProcessDay {
        #[arg(long)]
        device_id: Uuid,
        #[arg(long)]
        date: chrono::NaiveDate,
    },
    Owner {
        #[command(subcommand)]
        command: OwnerCommand,
    },
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
}

#[derive(Subcommand)]
enum OwnerCommand {
    Create(CreateOwnerArgs),
}

#[derive(Args)]
struct CreateOwnerArgs {
    #[arg(long)]
    display_name: String,
    #[arg(long)]
    timezone: String,
}

#[derive(Subcommand)]
enum DeviceCommand {
    Create(CreateDeviceArgs),
}

#[derive(Args)]
struct CreateDeviceArgs {
    #[arg(long)]
    owner_id: Uuid,
    #[arg(long)]
    name: String,
}

#[derive(Serialize)]
struct CreatedDevice {
    id: Uuid,
    owner_user_id: Uuid,
    name: String,
    token: String,
}

#[tokio::main]
async fn main() -> ExitCode {
    init_tracing();
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let database_url = cli.database_url.ok_or("LT_DATABASE_URL must be set")?;
    let pool = db::connect(&database_url).await?;
    db::migrate(&pool).await?;

    match cli.command {
        Command::Serve => serve(pool).await?,
        Command::ProcessDay { device_id, date } => {
            lifetrail_server::processing::queue_day(&pool, device_id, date).await?;
            while lifetrail_server::processing::process_next(&pool).await? {}
            print_json(&lifetrail_server::processing::status(&pool, device_id, date).await?)?;
        }
        Command::Owner {
            command: OwnerCommand::Create(args),
        } => print_json(&db::create_owner(&pool, &args.display_name, &args.timezone).await?)?,
        Command::Device {
            command: DeviceCommand::Create(args),
        } => {
            let token = generate_device_token();
            let device = db::create_device(&pool, args.owner_id, &args.name, &token).await?;
            print_json(&CreatedDevice {
                id: device.id,
                owner_user_id: device.owner_user_id,
                name: device.name,
                token,
            })?;
        }
    }
    Ok(())
}

async fn serve(pool: sqlx::PgPool) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let listener = tokio::net::TcpListener::bind(config.bind_addr).await?;
    let worker = tokio::spawn(lifetrail_server::processing::run(pool.clone()));
    tracing::info!(address = %config.bind_addr, "LifeTrail server listening");
    let result = axum::serve(
        listener,
        app::router(AppState { db: pool }, config.static_dir),
    )
    .await;
    worker.abort();
    result?;
    Ok(())
}

fn print_json<T: Serialize>(value: &T) -> Result<(), serde_json::Error> {
    println!("{}", serde_json::to_string(value)?);
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lifetrail_server=info,tower_http=info".into()),
        )
        .with_target(false)
        .init();
}
