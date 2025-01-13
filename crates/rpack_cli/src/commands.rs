use std::{io::Write, path::Path};

use clap::Subcommand;
use rpack_cli::{ImageFile, Spritesheet, TilemapGenerationConfig};

use rpack_cli::SaveImageFormat;

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Generates a tilemap
    Generate {
        /// Name of the tilemap to build, when no value is provided uses 'tilemap'
        #[clap(action)]
        name: Option<String>,
        /// size of the tilemap, default: 512
        #[arg(long)]
        size: Option<u32>,
        /// Image format
        #[clap(short, long)]
        format: Option<SaveImageFormat>,
    },
    /// Creates a tilemap generation config that can be used by this tool
    ConfigCreate {
        /// path of the config to create
        #[clap(action)]
        config_path: String,
        /// path of the tilemap to build, when no value is provided uses '/tilemap'
        #[clap(long)]
        output_path: Option<String>,
        /// size of the tilemap, default: 512
        #[arg(long)]
        size: Option<u32>,
        /// Image format, png by default
        #[clap(short, long)]
        format: Option<SaveImageFormat>,
        /// Asset sources path, argument can be passed multiple times
        #[clap(short, long)]
        source_paths: Vec<String>,
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
            Commands::Generate { name, size, format } => Self::generate_tilemap(name, size, format),
            Commands::ConfigCreate {
                config_path,
                output_path,
                size,
                format,
                source_paths,
            } => Self::create_config(config_path, output_path, size, format, source_paths),
            Commands::GenerateFromConfig { config_path } => {
                Self::generate_tilemap_from_config(config_path)
            }
        }
    }

    fn generate_tilemap(
        name: Option<String>,
        size: Option<u32>,
        format: Option<SaveImageFormat>,
    ) -> anyhow::Result<()> {
        let name = name.unwrap_or("tilemap".to_owned());
        let format = format.unwrap_or_default();
        let atlas_filename = format!("{}{}", name, format);
        let atlas_json_filename = format!("{}.rpack.json", name);
        let size = size.unwrap_or(512);

        let images: Vec<ImageFile> = glob::glob("**/*png")?
            .flatten()
            .flat_map(|f| ImageFile::at_path(&f, f.to_str().unwrap_or_default()))
            .collect();
        let spritesheet = Spritesheet::build(
            texture_packer::TexturePackerConfig {
                max_width: size,
                max_height: size,
                allow_rotation: false,
                force_max_dimensions: true,
                border_padding: 2,
                texture_padding: 2,
                texture_extrusion: 2,
                trim: false,
                texture_outlines: false,
            },
            &images,
            &atlas_filename,
        )?;

        if Path::new(&atlas_json_filename).exists() {
            std::fs::remove_file(&atlas_json_filename).expect("Could not remove the old file");
        }
        if Path::new(&atlas_filename).exists() {
            std::fs::remove_file(&atlas_filename).expect("Could not remove the old file");
        }
        match format {
            SaveImageFormat::Dds => {
                #[cfg(feature = "dds")]
                spritesheet.save_as_dds(&atlas_filename);
                #[cfg(not(feature = "dds"))]
                panic!("Program is compiled without support for dds. Compile it yourself with feature `dds` enabled.");
            }
            f => {
                spritesheet
                    .image_data
                    .save_with_format(&atlas_filename, f.into())?;
            }
        }
        let json = serde_json::to_string_pretty(&spritesheet.atlas_asset_json)?;
        let mut file = std::fs::File::create(&atlas_json_filename)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }
    fn create_config(
        config_path: String,
        output_path: Option<String>,
        size: Option<u32>,
        format: Option<SaveImageFormat>,
        source_paths: Vec<String>,
    ) -> Result<(), anyhow::Error> {
        let name = output_path.unwrap_or("tilemap".to_owned());
        let format = format.unwrap_or_default();
        let size = size.unwrap_or(512);
        let config = TilemapGenerationConfig {
            size,
            asset_paths: source_paths,
            output_path: name,
            format,
        };

        let json = serde_json::to_string_pretty(&config)?;
        let mut file = std::fs::File::create(format!("{}.rpack_gen.json", config_path)).unwrap();
        file.write_all(json.as_bytes())?;

        Ok(())
    }

    fn generate_tilemap_from_config(config_path: String) -> anyhow::Result<()> {
        let config_file = std::fs::read_to_string(&config_path)?;
        let config: TilemapGenerationConfig = serde_json::from_str(&config_file)?;

        let images: Vec<ImageFile> = config
            .asset_paths
            .iter()
            .flat_map(|path| {
                let pattern = format!("{}/*png", path);
                glob::glob(&pattern)
                    .unwrap()
                    .flatten()
                    .flat_map(|f| ImageFile::at_path(&f, f.to_str().unwrap_or_default()))
            })
            .collect();
        let atlas_filename = format!("{}{}", config.output_path, config.format);
        let atlas_json_filename = format!("{}.rpack.json", config.output_path);
        let spritesheet = Spritesheet::build(
            texture_packer::TexturePackerConfig {
                max_width: config.size,
                max_height: config.size,
                allow_rotation: false,
                force_max_dimensions: true,
                border_padding: 2,
                texture_padding: 2,
                texture_extrusion: 2,
                trim: false,
                texture_outlines: false,
            },
            &images,
            &atlas_filename,
        )?;

        if Path::new(&atlas_json_filename).exists() {
            std::fs::remove_file(&atlas_json_filename).expect("Could not remove the old file");
        }
        if Path::new(&atlas_filename).exists() {
            std::fs::remove_file(&atlas_filename).expect("Could not remove the old file");
        }
        match config.format {
            SaveImageFormat::Dds => {
                #[cfg(feature = "dds")]
                spritesheet.save_as_dds(&atlas_filename);
                #[cfg(not(feature = "dds"))]
                panic!("Program is compiled without support for dds. Compile it yourself with feature `dds` enabled.");
            }
            f => {
                spritesheet
                    .image_data
                    .save_with_format(&atlas_filename, f.into())?;
            }
        }
        let json = serde_json::to_string_pretty(&spritesheet.atlas_asset_json)?;
        let mut file = std::fs::File::create(&atlas_json_filename)?;
        file.write_all(json.as_bytes())?;

        Ok(())
    }
}
