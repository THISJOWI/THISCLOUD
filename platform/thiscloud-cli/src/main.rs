mod commands;

use clap::{Parser, Subcommand};
use commands::image::ImageCommands;
use commands::marketplace::MarketplaceCommands;
use commands::network::NetworkCommands;
use commands::node::NodeCommands;
use commands::storage::StorageCommands;
use commands::vm::VmCommands;

#[derive(Parser)]
#[command(name = "thiscloud")]
#[command(about = "THISCLOUD Hypervisor OS CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize THISCLOUD on this node
    Init {
        /// IP address of this node
        #[arg(long)]
        ip: Option<String>,
        /// Role: master or worker
        #[arg(long, default_value = "master")]
        role: String,
    },
    /// Show cluster status
    Status,
    /// Join an existing cluster
    Join {
        /// IP address of master node
        #[arg(long)]
        master: String,
        /// IP address of this node
        #[arg(long)]
        ip: Option<String>,
    },
    /// Manage virtual machines
    Vm {
        #[command(subcommand)]
        command: VmCommands,
    },
    /// Manage virtual networks
    Network {
        #[command(subcommand)]
        command: NetworkCommands,
    },
    /// Manage storage pools
    Storage {
        #[command(subcommand)]
        command: StorageCommands,
    },
    /// Manage marketplace apps
    Marketplace {
        #[command(subcommand)]
        command: MarketplaceCommands,
    },
    /// Manage cluster nodes
    Node {
        #[command(subcommand)]
        command: NodeCommands,
    },
    /// Manage VM images and templates
    Image {
        #[command(subcommand)]
        command: ImageCommands,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Init { ip, role } => commands::run_init(ip, &role),
        Commands::Status => commands::run_status().await,
        Commands::Join { master, ip } => commands::run_join(&master, ip.as_deref()).await,
        Commands::Vm { command } => commands::run_vm_command(command).await,
        Commands::Network { command } => commands::run_network_command(command).await,
        Commands::Storage { command } => commands::run_storage_command(command).await,
        Commands::Marketplace { command } => commands::run_marketplace_command(command).await,
        Commands::Node { command } => commands::run_node_command(command).await,
        Commands::Image { command } => commands::run_image_command(command).await,
    }
}
