use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
        name = "knowledge-cli",
        version,
        about = "Crea, vincula, consulta y valida registros JSON-LD",
        long_about = "Crea, vincula, consulta y valida registros de conocimiento en formato JSON-LD.\n\
Incluye operaciones de grafo para explorar relaciones (links, neighbors, path).",
        after_help = "USOS RAPIDOS POR COMANDO:\n\
    create decision   --title 'Adoptar Postgres' --rationale 'Necesitamos ACID' --domain payments\n\
    create fact       --observation 'Cola X saturada' --domain payments\n\
    create assumption --assumption-statement 'Trafico estable' --domain payments\n\
    create meeting    --title 'Payments weekly' --date 2026-03-31 --domain payments\n\
    create action     --title 'Afinar worker' --status todo --domain payments\n\
    create decision   --title 'DB' --rationale 'ACID' --classification tactical --related-files 'src/db.rs,docs/adr.md'\n\
    create fact       --observation 'Error 500' --evidence-json '{\"type\":\"issue\",\"reference\":\"#123\"}'\n\
    create person     --name 'Alice Example' --role Engineer\n\
    link              --from <ID> --to <ID> --relation relatesTo\n\
    validate          --file knowledge/payments/registro.jsonld\n\
    show              --id <ID>\n\
    list              --domain payments --type Decision\n\
    links             --from <ID>\n\
    neighbors         --id <ID> --depth 2\n\
    search            --query postgres --in Decision,Fact\n\
    validate-graph    --report json\n\
    path              --from <ID> --to <ID> --max-depth 10\n\
    extended-help\n\
\n\
SKILL CREATE (TODAS LAS VARIANTES):\n\
    create decision   -> requeridos: --title --rationale\n\
    create fact       -> requerido : --observation\n\
    create assumption -> requerido : --assumption-statement\n\
    create meeting    -> requeridos: --title --date\n\
    create action     -> requerido : --title\n\
    create person     -> requerido : --name\n\
\n\
CAMPOS OPCIONALES DE ONTOLOGIA (create):\n\
    --classification foundational|tactical|observational\n\
    --related-files a,b,c\n\
    --evidence-json '{\"type\":\"issue\",\"reference\":\"#123\"}' (repetible)\n\
    --outcomes-json '{\"status\":\"success\",\"notes\":\"ok\"}' (repetible)\n\
    --provenance-json '{\"source_tool\":\"cli\",\"recorded_via\":\"manual\"}'\n\
    create fact   extra: --related-components compA,compB\n\
    create action extra: --outcome-json '{\"status\":\"partial\"}'\n\
    aliases       : --project == --domain, --body == --description\n\
\n\
CONTRAEJEMPLOS COMUNES:\n\
    create decision sin --title o sin --rationale\n\
    create action con --status invalido (todo,in-progress,blocked,done,cancelled)\n\
    create meeting con --date invalida (usa YYYY-MM-DD o ISO-8601)"
)]
pub struct Cli {
        #[arg(long, default_value = "knowledge", help = "Directorio base de conocimiento")]
    pub knowledge_dir: PathBuf,

        #[arg(
                long,
                num_args = 0..=1,
                default_missing_value = "general",
                help = "Imprime guia de uso: general o por dominio (ej: --skill payments)"
        )]
        pub skill: Option<String>,

    #[command(subcommand)]
        pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
        #[command(about = "Crear un registro JSON-LD por tipo")]
    Create {
        #[command(subcommand)]
        kind: Box<CreateCommands>,
    },

        #[command(about = "Vincular dos registros por relacion")]
    Link(LinkArgs),

        #[command(about = "Validar sintaxis y reglas de un archivo JSON-LD")]
    Validate(ValidateArgs),

        #[command(about = "Mostrar un registro completo por ID")]
    Show(ShowArgs),

        #[command(about = "Listar registros con filtros por tipo, dominio, classification, tags y fechas")]
    List(ListArgs),

        #[command(about = "Consultar relaciones por origen y/o destino")]
    Links(QueryLinksArgs),

        #[command(about = "Obtener vecinos de un nodo en el grafo de relaciones")]
    Neighbors(NeighborsArgs),

        #[command(about = "Buscar texto libre en registros")]
    Search(SearchArgs),

        #[command(about = "Validar integridad del grafo (links rotos, orfandad, ciclos)")]
    ValidateGraph(ValidateGraphArgs),

        #[command(about = "Buscar camino entre dos IDs")]
    Path(PathArgs),

        #[command(about = "Mostrar documentacion extendida estilo Linux")]
    ExtendedHelp,
}

#[derive(Debug, Subcommand)]
pub enum CreateCommands {
        #[command(about = "Crear una Decision")]
    Decision(CreateDecisionArgs),

        #[command(about = "Crear un Fact")]
    Fact(CreateFactArgs),

        #[command(about = "Crear una Assumption")]
    Assumption(CreateAssumptionArgs),

        #[command(about = "Crear un Meeting")]
    Meeting(CreateMeetingArgs),

        #[command(about = "Crear un Action")]
    Action(CreateActionArgs),

        #[command(about = "Crear un Person")]
    Person(CreatePersonArgs),
}

#[derive(Debug, Args)]
pub struct CommonCreateArgs {
    #[arg(long)]
    pub id: Option<String>,

    #[arg(long)]
    pub recorded_at: Option<String>,

    #[arg(long, visible_alias = "project")]
    pub domain: Option<String>,

    #[arg(long, visible_alias = "body")]
    pub description: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    #[arg(long)]
    pub author: Option<String>,

    #[arg(long)]
    pub status: Option<String>,

    #[arg(long)]
    pub confidence: Option<f64>,

    #[arg(long)]
    pub impact: Option<String>,

    #[arg(long)]
    pub classification: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub related_files: Vec<String>,

    #[arg(long = "evidence-json")]
    pub evidence_json: Vec<String>,

    #[arg(long = "outcomes-json")]
    pub outcomes_json: Vec<String>,

    #[arg(long = "provenance-json")]
    pub provenance_json: Option<String>,

    #[arg(long, default_value = "./context.jsonld")]
    pub context: String,
}

#[derive(Debug, Args)]
pub struct CreateDecisionArgs {
    #[command(flatten)]
    pub common: CommonCreateArgs,

    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub rationale: String,

    #[arg(long, value_delimiter = ',')]
    pub options_considered: Vec<String>,

    #[arg(long)]
    pub chosen_option: Option<String>,

    #[arg(long)]
    pub effective_from: Option<String>,

    #[arg(long)]
    pub consequences: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub impacted_components: Vec<String>,
}

#[derive(Debug, Args)]
pub struct CreateFactArgs {
    #[command(flatten)]
    pub common: CommonCreateArgs,

    #[arg(long)]
    pub observation: String,

    #[arg(long, value_delimiter = ',')]
    pub related_components: Vec<String>,
}

#[derive(Debug, Args)]
pub struct CreateAssumptionArgs {
    #[command(flatten)]
    pub common: CommonCreateArgs,

    #[arg(long)]
    pub assumption_statement: String,

    #[arg(long, value_delimiter = ',')]
    pub tests_needed: Vec<String>,

    #[arg(long)]
    pub expire_at: Option<String>,
}

#[derive(Debug, Args)]
pub struct CreateMeetingArgs {
    #[command(flatten)]
    pub common: CommonCreateArgs,

    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub date: String,

    #[arg(long)]
    pub location: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub participants: Vec<String>,

    #[arg(long)]
    pub minutes: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub decisions_made: Vec<String>,

    #[arg(long, value_delimiter = ',')]
    pub actions: Vec<String>,
}

#[derive(Debug, Args)]
pub struct CreateActionArgs {
    #[command(flatten)]
    pub common: CommonCreateArgs,

    #[arg(long)]
    pub title: String,

    #[arg(long)]
    pub assigned_to: Option<String>,

    #[arg(long)]
    pub due_date: Option<String>,

    #[arg(long)]
    pub parent_decision: Option<String>,

    #[arg(long = "outcome-json")]
    pub outcome_json: Option<String>,
}

#[derive(Debug, Args)]
pub struct CreatePersonArgs {
    #[arg(long)]
    pub id: Option<String>,

    #[arg(long)]
    pub name: String,

    #[arg(long)]
    pub role: Option<String>,

    #[arg(long)]
    pub contact: Option<String>,

    #[arg(long, default_value = "./context.jsonld")]
    pub context: String,
}

#[derive(Debug, Args)]
pub struct LinkArgs {
    #[arg(long)]
    pub from: String,

    #[arg(long)]
    pub to: String,

    #[arg(long)]
    pub relation: String,

    #[arg(long)]
    pub rationale: Option<String>,

    #[arg(long)]
    pub strength: Option<f64>,
}

#[derive(Debug, Args)]
pub struct ValidateArgs {
    #[arg(long)]
    pub file: PathBuf,
}

#[derive(Debug, Args)]
pub struct ShowArgs {
    #[arg(long)]
    pub id: String,
}

#[derive(Debug, Args)]
pub struct ListArgs {
    #[arg(long = "type")]
    pub record_type: Option<String>,

    #[arg(long)]
    pub domain: Option<String>,

    #[arg(long)]
    pub classification: Option<String>,

    #[arg(long, value_delimiter = ',')]
    pub tags: Vec<String>,

    #[arg(long)]
    pub status: Option<String>,

    #[arg(long)]
    pub author: Option<String>,

    #[arg(long)]
    pub from_date: Option<String>,

    #[arg(long)]
    pub to_date: Option<String>,

    #[arg(long, default_value_t = 100)]
    pub limit: usize,

    #[arg(long, default_value_t = 0)]
    pub offset: usize,
}

#[derive(Debug, Args)]
pub struct QueryLinksArgs {
    #[arg(long)]
    pub from: Option<String>,

    #[arg(long)]
    pub to: Option<String>,

    #[arg(long)]
    pub relation: Option<String>,
}

#[derive(Debug, Args)]
pub struct NeighborsArgs {
    #[arg(long)]
    pub id: String,

    #[arg(long, default_value_t = 1)]
    pub depth: usize,

    #[arg(long)]
    pub relation: Option<String>,
}

#[derive(Debug, Args)]
pub struct SearchArgs {
    #[arg(long)]
    pub query: String,

    #[arg(long = "in", value_delimiter = ',')]
    pub in_types: Vec<String>,

    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

#[derive(Debug, Args)]
pub struct ValidateGraphArgs {
    #[arg(long, default_value = "text")]
    pub report: String,
}

#[derive(Debug, Args)]
pub struct PathArgs {
    #[arg(long)]
    pub from: String,

    #[arg(long)]
    pub to: String,

    #[arg(long)]
    pub relation: Option<String>,

    #[arg(long, default_value_t = 10)]
    pub max_depth: usize,
}
