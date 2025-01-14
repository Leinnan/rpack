use std::io::Write;

use clap::Subcommand;
use rpack_cli::TilemapGenerationConfig;

use rpack_cli::SaveImageFormat;

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Generates a tilemap
    Generate {
        /// Name of the tilemap to build, when no value is provided uses 'tilemap'
        #[clap(action)]
        name: Option<String>,
        /// size of the tilemap, default: 2048
        #[arg(long)]
        size: Option<u32>,
        /// Image format
        #[clap(short, long)]
        format: Option<SaveImageFormat>,
        /// Asset sources path, argument can be passed multiple times
        #[clap(short, long)]
        source_paths: Vec<String>,
        /// Size of the padding between frames in pixel. Default value is `2`
        texture_padding: Option<u32>,
        /// Size of the padding on the outer edge of the packed image in pixel. Default value is `0`.
        border_padding: Option<u32>,
    },
    /// Creates a tilemap generation config
    ConfigCreate {
        /// path of the config to create
        #[clap(action)]
        config_path: String,
        /// path of the tilemap to build, when no value is provided uses '/tilemap'
        #[clap(long)]
        output_path: Option<String>,
        /// size of the tilemap, default: 2048
        #[arg(long)]
        size: Option<u32>,
        /// Image format, png by default
        #[clap(short, long)]
        format: Option<SaveImageFormat>,
        /// Asset sources path, argument can be passed multiple times
        #[clap(short, long)]
        source_paths: Vec<String>,
        /// Size of the padding between frames in pixel. Default value is `2`
        texture_padding: Option<u32>,
        /// Size of the padding on the outer edge of the packed image in pixel. Default value is `0`.
        border_padding: Option<u32>,
    },
    /// Generates a tilemap from config
    GenerateFromConfig {
        /// path of the config to use
        #[clap(action)]
        config_path: String,
    },
}
impl Commands {
    pub(crate) fn run(&self) -> anyhow::Result<()> {
        match self.clone() {
            Commands::Generate {
                name,
                size,
                format,
                source_paths,
                texture_padding,
                border_padding,
            } => Self::generate_tilemap(
                name,
                size,
                format,
                source_paths,
                texture_padding,
                border_padding,
            ),
            Commands::ConfigCreate {
                config_path,
                output_path,
                size,
                format,
                source_paths,
                texture_padding,
                border_padding,
            } => Self::create_config(
                config_path,
                output_path,
                size,
                format,
                source_paths,
                texture_padding,
                border_padding,
            ),
            Commands::GenerateFromConfig { config_path } => {
                Self::generate_tilemap_from_config(config_path)
            }
        }
    }

    fn generate_tilemap(
        name: Option<String>,
        size: Option<u32>,
        format: Option<SaveImageFormat>,
        source_paths: Vec<String>,
        texture_padding: Option<u32>,
        border_padding: Option<u32>,
    ) -> anyhow::Result<()> {
        let name = name.unwrap_or("tilemap".to_owned());
        let source_paths = if source_paths.is_empty() {
            vec![".".to_owned()]
        } else {
            source_paths
        };
        let config = TilemapGenerationConfig {
            asset_patterns: source_paths,
            output_path: name,
            format,
            size,
            texture_padding,
            border_padding,
            ..Default::default()
        };

        config.generate()
    }

    fn create_config(
        config_path: String,
        output_path: Option<String>,
        size: Option<u32>,
        format: Option<SaveImageFormat>,
        source_paths: Vec<String>,
        texture_padding: Option<u32>,
        border_padding: Option<u32>,
    ) -> Result<(), anyhow::Error> {
        let name = output_path.unwrap_or("tilemap".to_owned());

        let config = TilemapGenerationConfig {
            size,
            asset_patterns: source_paths,
            output_path: name,
            format,
            texture_padding,
            border_padding,
            ..Default::default()
        };

        let json = serde_json::to_string_pretty(&config)?;
        let mut file = std::fs::File::create(format!("{}.rpack_gen.json", config_path)).unwrap();
        file.write_all(json.as_bytes())?;

        Ok(())
    }

    fn generate_tilemap_from_config(config_path: String) -> anyhow::Result<()> {
        let config = TilemapGenerationConfig::read_from_file(config_path)?;
        config.generate()
    }
}
