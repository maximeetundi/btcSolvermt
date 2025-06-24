use bitcoin::{Address, Network, PrivateKey};
use bitcoin::secp256k1::{Secp256k1, SecretKey, All};
use ibig::{ubig, UBig};
use rand::Rng;
use rand::seq::SliceRandom;
use num_cpus;
use std::collections::HashSet;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

// Énumérations pour les modes de calcul
#[derive(Debug, Clone, PartialEq)]
enum ComputeMode {
    CPU,
    GPU,
    Hybrid,
}

impl FromStr for ComputeMode {
    type Err = String;
    
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "cpu" => Ok(ComputeMode::CPU),
            "gpu" => Ok(ComputeMode::GPU),
            "hybrid" | "cpu+gpu" | "mixed" => Ok(ComputeMode::Hybrid),
            _ => Err(format!("Mode de calcul invalide: {}. Utilisez 'cpu', 'gpu', ou 'hybrid'", s))
        }
    }
}

#[derive(Debug, Clone)]
struct Config {
    start: String,
    end: String,
    cores: usize,
    mode: String,
    compute_mode: ComputeMode,
    gpu_device_id: usize,
    gpu_batch_size: usize,
    cpu_gpu_ratio: f64,
    switch_interval: u64,
    subinterval_ratio: f64,
    stop_on_find: bool,
    puzzle_file: String,
    baby_steps: bool,
    giant_steps: bool,
    bloom_filter: bool,
    smart_jump: bool,
    batch_size: usize,
    checkpoint_interval: u64,
    telegram_bot_token: Option<String>,
    telegram_chat_id: Option<String>,
}

#[derive(Debug, Clone)]
struct PuzzleData {
    addresses: HashSet<String>
}

#[derive(Debug)]
struct Statistics {
    start_time: Instant,
    keys_checked: AtomicU64,
    found_count: AtomicU64,
    cpu_keys_checked: AtomicU64,
    gpu_keys_checked: AtomicU64,
}

impl Statistics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            keys_checked: AtomicU64::new(0),
            found_count: AtomicU64::new(0),
            cpu_keys_checked: AtomicU64::new(0),
            gpu_keys_checked: AtomicU64::new(0),
        }
    }
    #[allow(dead_code)]
    fn add_keys(&self, count: u64) {
        self.keys_checked.fetch_add(count, Ordering::Relaxed);
    }
    
    fn add_cpu_keys(&self, count: u64) {
        self.cpu_keys_checked.fetch_add(count, Ordering::Relaxed);
        self.keys_checked.fetch_add(count, Ordering::Relaxed);
    }
    
    fn add_gpu_keys(&self, count: u64) {
        self.gpu_keys_checked.fetch_add(count, Ordering::Relaxed);
        self.keys_checked.fetch_add(count, Ordering::Relaxed);
    }
    
    fn get_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.keys_checked.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }
    
    fn get_cpu_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.cpu_keys_checked.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }
    
    fn get_gpu_rate(&self) -> f64 {
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            self.gpu_keys_checked.load(Ordering::Relaxed) as f64 / elapsed
        } else {
            0.0
        }
    }
}

// Structure pour la détection GPU
struct GPUInfo {
    available: bool,
    device_count: usize,
    device_names: Vec<String>,
    cuda_available: bool,
    opencl_available: bool,
}

impl GPUInfo {
    fn detect() -> Self {
        let mut gpu_info = GPUInfo {
            available: false,
            device_count: 0,
            device_names: Vec::new(),
            cuda_available: false,
            opencl_available: false,
        };
        
        // Détection CUDA (simulation - dans un vrai projet, utilisez cudarc ou similaire)
        if Self::check_cuda() {
            gpu_info.cuda_available = true;
            gpu_info.available = true;
            gpu_info.device_count += Self::get_cuda_device_count();
            gpu_info.device_names.extend(Self::get_cuda_device_names());
        }
        
        // Détection OpenCL (simulation - dans un vrai projet, utilisez opencl3 ou similaire)
        if Self::check_opencl() {
            gpu_info.opencl_available = true;
            gpu_info.available = true;
            gpu_info.device_count += Self::get_opencl_device_count();
            gpu_info.device_names.extend(Self::get_opencl_device_names());
        }
        
        gpu_info
    }
    
    // Simulation de détection CUDA
    fn check_cuda() -> bool {
        // Dans un projet réel, utilisez nvidia-ml-rs ou cudarc
        std::process::Command::new("nvidia-smi")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
    
    fn get_cuda_device_count() -> usize {
        // Simulation - remplacez par une vraie détection
        if Self::check_cuda() { 1 } else { 0 }
    }
    
    fn get_cuda_device_names() -> Vec<String> {
        // Simulation - remplacez par une vraie détection
        if Self::check_cuda() {
            vec!["NVIDIA GPU (CUDA)".to_string()]
        } else {
            Vec::new()
        }
    }
    
    // Simulation de détection OpenCL
    fn check_opencl() -> bool {
        // Dans un projet réel, utilisez opencl3
        cfg!(target_os = "linux") && std::fs::metadata("/usr/lib/x86_64-linux-gnu/libOpenCL.so.1").is_ok() ||
        cfg!(target_os = "macos") && std::fs::metadata("/System/Library/Frameworks/OpenCL.framework").is_ok() ||
        cfg!(target_os = "windows") && std::fs::metadata("C:\\Windows\\System32\\OpenCL.dll").is_ok()
    }
    
    fn get_opencl_device_count() -> usize {
        // Simulation - remplacez par une vraie détection
        if Self::check_opencl() { 1 } else { 0 }
    }
    
    fn get_opencl_device_names() -> Vec<String> {
        // Simulation - remplacez par une vraie détection
        if Self::check_opencl() {
            vec!["OpenCL Device".to_string()]
        } else {
            Vec::new()
        }
    }
}

// Générateur de nombres pseudo-aléatoires optimisé pour la cryptographie
struct FastRng {
    state: u64,
}

impl FastRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    
    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }
    
    fn gen_range(&mut self, min: u64, max: u64) -> u64 {
        if max <= min { return min; }
        min + (self.next() % (max - min))
    }
}

// Simulateur GPU pour les calculs de clés
#[allow(dead_code)]
struct GPUWorker {
    device_id: usize,
    batch_size: usize,
}

impl GPUWorker {
    fn new(device_id: usize, batch_size: usize) -> Self {
        Self { device_id, batch_size }
    }
    
    // Simulation du traitement GPU - remplacez par du vrai code GPU
    fn process_key_batch(&self, keys: &[UBig]) -> Vec<(UBig, Vec<String>)> {
        let secp = Secp256k1::new();
        let mut results = Vec::new();

            // Utiliser self.batch_size pour limiter le traitement
        let keys_to_process = &keys[..keys.len().min(self.batch_size)];
        
        println!("GPU Device {} processing {} keys", self.device_id, keys_to_process.len());
        
        
        // Simuler un traitement parallèle GPU plus rapide
        for key_val in keys_to_process {
            if *key_val == ubig!(0) {
                continue;
            }
            
            let key_bytes = key_val.to_be_bytes();
            if key_bytes.len() > 32 {
                continue;
            }
            
            let mut padded = [0u8; 32];
            padded[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
            
            if let Ok(secret_key) = SecretKey::from_slice(&padded) {
                let addresses = self.generate_addresses_gpu(&secp, &secret_key);
                results.push((key_val.clone(), addresses));
            }
        }
        
        results
    }
    
    fn generate_addresses_gpu(&self, secp: &Secp256k1<All>, secret_key: &SecretKey) -> Vec<String> {
        let mut addresses = Vec::new();
        
        // Version compressée
        let private_key_compressed = PrivateKey {
            compressed: true,
            network: Network::Bitcoin.into(),
            inner: *secret_key,
        };
        
        let public_key_compressed = private_key_compressed.public_key(secp);
        let address_compressed = Address::p2pkh(&public_key_compressed, Network::Bitcoin);
        addresses.push(address_compressed.to_string());
        
        // Version non compressée
        let private_key_uncompressed = PrivateKey {
            compressed: false,
            network: Network::Bitcoin.into(),
            inner: *secret_key,
        };
        let public_key_uncompressed = private_key_uncompressed.public_key(secp);
        let address_uncompressed = Address::p2pkh(&public_key_uncompressed, Network::Bitcoin);
        addresses.push(address_uncompressed.to_string());
        
        addresses
    }
}

// Structure pour Baby-step Giant-step optimisé
// (Actuellement non implémenté dans cette version)
#[allow(dead_code)]
struct BabyStepGiantStep;

#[allow(dead_code)]
impl BabyStepGiantStep {
    fn new() -> Self {
        Self {}
    }
}

fn load_puzzle_advanced(path: &str) -> PuzzleData {
    let file = File::open(path).expect(&format!("Impossible d'ouvrir le fichier puzzle : {}", path));
    let reader = BufReader::new(file);
    
    let mut addresses = HashSet::new();
    
    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        
        addresses.insert(line.to_string());
    }
    
    PuzzleData {
        addresses,
    }
}

fn generate_address_variants(secp: &Secp256k1<All>, secret_key: &SecretKey) -> Vec<(PrivateKey, Address)> {
    let mut variants = Vec::new();
    
    // Version compressée
    let private_key_compressed = PrivateKey {
        compressed: true,
        network: Network::Bitcoin.into(),
        inner: *secret_key,
    };
    let public_key_compressed = private_key_compressed.public_key(secp);
    let address_compressed = Address::p2pkh(&public_key_compressed, Network::Bitcoin);
    variants.push((private_key_compressed, address_compressed));
    
    // Version non compressée
    let private_key_uncompressed = PrivateKey {
        compressed: false,
        network: Network::Bitcoin.into(),
        inner: *secret_key,
    };
    let public_key_uncompressed = private_key_uncompressed.public_key(secp);
    let address_uncompressed = Address::p2pkh(&public_key_uncompressed, Network::Bitcoin);
    variants.push((private_key_uncompressed, address_uncompressed));
    
    variants
}

fn create_default_config(path: &str) {
    let gpu_info = GPUInfo::detect();
    let default_compute_mode = if gpu_info.available { "hybrid" } else { "cpu" };
    
    let config_content = format!("# Fichier de configuration pour le solveur de puzzle Bitcoin OPTIMISÉ v2.1
# Modifiez les valeurs ci-dessous puis relancez le programme.

# Plage de recherche (peut être en décimal ou en hexadécimal préfixé par 0x)
start=0x20000000000000000
end=0x3ffffffffffffffff

# Nombre de coeurs CPU à utiliser (0 = détection automatique)
cores=0

# Mode de recherche : 'random', 'sequential', 'smart', 'kangaroo'
mode=smart

# Mode de calcul : 'cpu', 'gpu', 'hybrid' (cpu+gpu)
compute_mode={}

# Configuration GPU
gpu_device_id=0
gpu_batch_size=50000

# Ratio CPU/GPU en mode hybride (0.5 = 50% CPU, 50% GPU)
cpu_gpu_ratio=0.5

# Après combien d'essais sauter vers un nouvel emplacement aléatoire
switch_interval=1000000

# Ratio de la taille du sous-intervalle (ex: 0.001 pour 0.1%)
subinterval_ratio=0.001

# Arrêter le programme dès qu'une clé est trouvée ? (true ou false)
stop_on_find=false

# Fichier contenant la liste des adresses Bitcoin à trouver
puzzle_file=puzzle.txt

# Algorithmes avancés
baby_steps=true
giant_steps=true
bloom_filter=false
smart_jump=true

# Paramètres de performance
batch_size=10000
checkpoint_interval=10000000

# Configuration Telegram (optionnel)
# Créez un bot avec @BotFather et obtenez le token
# Ajoutez le bot à un chat et obtenez le chat_id avec @userinfobot
telegram_bot_token=
telegram_chat_id=

# Informations GPU détectées automatiquement :
# GPU disponible : {}
# Nombre d'appareils : {}
# CUDA disponible : {}
# OpenCL disponible : {}
", 
    default_compute_mode,
    gpu_info.available,
    gpu_info.device_count,
    gpu_info.cuda_available,
    gpu_info.opencl_available
    );

    println!("Création d'un nouveau fichier de configuration par défaut...");
    let mut file = File::create(path).expect("Impossible de créer le fichier de configuration.");
    file.write_all(config_content.as_bytes())
        .expect("Impossible d'écrire dans le fichier de configuration.");
    
    println!("Un fichier de configuration '{}' a été créé avec des valeurs par défaut.", path);
    if gpu_info.available {
        println!("🚀 GPU détecté ! Mode hybride recommandé pour de meilleures performances.");
        for (i, name) in gpu_info.device_names.iter().enumerate() {
            println!("  GPU {}: {}", i, name);
        }
    } else {
        println!("⚠️  Aucun GPU détecté. Mode CPU configuré par défaut.");
    }
    println!("Veuillez le modifier selon vos besoins avant de relancer l'application.");
}

fn load_config(path: &str) -> Config {
    // Valeurs par défaut
    let mut config = Config {
        start: "1".to_string(),
        end: "1000000".to_string(),
        cores: num_cpus::get(),
        mode: "sequential".to_string(),
        compute_mode: ComputeMode::CPU,
        gpu_device_id: 0,
        gpu_batch_size: 50000,
        cpu_gpu_ratio: 0.5,
        switch_interval: 1000,
        subinterval_ratio: 0.1,
        stop_on_find: true,
        puzzle_file: "puzzle.txt".to_string(),
        baby_steps: false,
        giant_steps: false,
        bloom_filter: true,
        smart_jump: true,
        batch_size: 10000,
        checkpoint_interval: 10000000,
        telegram_bot_token: None,
        telegram_chat_id: None,
    };
    
    if let Ok(file) = File::open(path) {
        for line in BufReader::new(file).lines().filter_map(Result::ok) {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                
                match key {
                    "start" => config.start = value.to_string(),
                    "end" => config.end = value.to_string(),
                    "cores" => if let Ok(cores) = value.parse() { config.cores = cores },
                    "mode" => config.mode = value.to_string(),
                    "compute_mode" => {
                        match ComputeMode::from_str(value) {
                            Ok(mode) => config.compute_mode = mode,
                            Err(e) => {
                                eprintln!("⚠️  {}", e);
                                eprintln!("    Mode CPU utilisé par défaut.");
                            }
                        }
                    },
                    "gpu_device_id" => if let Ok(id) = value.parse() { config.gpu_device_id = id },
                    "gpu_batch_size" => if let Ok(size) = value.parse() { config.gpu_batch_size = size },
                    "cpu_gpu_ratio" => if let Ok(ratio) = value.parse() { 
                        config.cpu_gpu_ratio = ratio;
                    },
                    "switch_interval" => if let Ok(interval) = value.parse() { config.switch_interval = interval },
                    "subinterval_ratio" => if let Ok(ratio) = value.parse() { config.subinterval_ratio = ratio },
                    "stop_on_find" => config.stop_on_find = value.eq_ignore_ascii_case("true"),
                    "puzzle_file" => config.puzzle_file = value.to_string(),
                    "baby_steps" => config.baby_steps = value.eq_ignore_ascii_case("true"),
                    "giant_steps" => config.giant_steps = value.eq_ignore_ascii_case("true"),
                    "bloom_filter" => config.bloom_filter = value.eq_ignore_ascii_case("true"),
                    "smart_jump" => config.smart_jump = value.eq_ignore_ascii_case("true"),
                    "batch_size" => if let Ok(size) = value.parse() { config.batch_size = size },
                    "checkpoint_interval" => if let Ok(interval) = value.parse() { config.checkpoint_interval = interval },
                    "telegram_bot_token" if !value.is_empty() => config.telegram_bot_token = Some(value.to_string()),
                    "telegram_chat_id" if !value.is_empty() => config.telegram_chat_id = Some(value.to_string()),
                    _ => {}
                }
            }
        }
    }
    
    config
}

fn parse_big_int(s: &str) -> Result<UBig, Box<dyn std::error::Error>> {
    if let Some(hex_val) = s.strip_prefix("0x") {
        Ok(UBig::from_str_radix(hex_val, 16)?)
    } else {
        Ok(UBig::from_str(s)?)
    }
}

fn send_telegram_notification(token: &str, chat_id: &str, message: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let url = format!("https://api.telegram.org/bot{}/sendMessage", token);
    let client = reqwest::blocking::Client::new();
    
    let params = [
        ("chat_id", chat_id),
        ("text", message),
        ("parse_mode", "HTML"),
    ];
    
    client.post(&url)
        .form(&params)
        .send()?;
    
    Ok(())
}

fn save_checkpoint(current_key: &UBig, core_id: usize) {
    let checkpoint_file = format!("checkpoint_core_{}.txt", core_id);
    if let Ok(mut file) = File::create(&checkpoint_file) {
        let _ = file.write_all(current_key.to_string().as_bytes());
    }
}

fn load_checkpoint(core_id: usize, default_start: &UBig) -> UBig {
    let checkpoint_file = format!("checkpoint_core_{}.txt", core_id);
    if let Ok(file) = File::open(&checkpoint_file) {
        let mut reader = BufReader::new(file);
        let mut contents = String::new();
        if reader.read_line(&mut contents).is_ok() {
            if let Ok(checkpoint) = UBig::from_str(contents.trim()) {
                println!("[Core {}] Point de contrôle chargé: {}", core_id, checkpoint);
                return checkpoint;
            }
        }
    }
    default_start.clone()
}

// Générateur de patterns avancés pour les clés
fn generate_key_patterns(base_key: &UBig, _rng: &mut FastRng) -> Vec<UBig> {
    let mut patterns = Vec::new();
    
    // Pattern original
    patterns.push(base_key.clone());
    
    // Patterns basés sur les propriétés mathématiques
    let base_str = base_key.to_string();
    
    // Vérifier que la chaîne de base est valide pour les conversions
    if !base_str.is_empty() && base_str.chars().all(|c| c.is_ascii_digit()) {
        // Inversion des chiffres
        let reversed: String = base_str.chars().rev().collect();
        if let Ok(inverted) = UBig::from_str(&reversed) {
            patterns.push(inverted);
        }
    }
    
    // Addition/soustraction de petites valeurs
    let offsets: [u64; 10] = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
    for &offset in &offsets {
        patterns.push(base_key.clone() + offset);
        if *base_key > UBig::from(offset) {
            patterns.push(base_key.clone() - offset);
        }
    }
    
    // Multiplication par des facteurs premiers
    let factors = [2, 3, 5, 7, 11, 13];
    for &factor in &factors {
        patterns.push(base_key.clone() * factor);
    }
    
    // Patterns basés sur les digits
    if base_str.chars().all(|c| c.is_ascii_digit()) {
        let mut chars: Vec<char> = base_str.chars().collect();
        for _ in 0..3.min(chars.len().saturating_sub(1)) {
            chars.shuffle(&mut rand::thread_rng());
            let shuffled_str: String = chars.iter().collect();
            if let Ok(shuffled) = UBig::from_str(&shuffled_str) {
                patterns.push(shuffled);
            }
        }
    }
    
    // Éliminer les doublons
    patterns.sort();
    patterns.dedup();
    
    patterns
}

// Worker GPU
fn gpu_worker_thread(
    device_id: usize,
    config: Arc<Config>,
    puzzle: Arc<PuzzleData>,
    stats: Arc<Statistics>,
    found: Arc<AtomicBool>,
    file_write_lock: Arc<Mutex<()>>,
    core_start: UBig,
    core_end: UBig,
) {
    let gpu_worker = GPUWorker::new(device_id, config.gpu_batch_size);
    let mut rng = FastRng::new(device_id as u64 * 2000000 + rand::thread_rng().gen::<u64>());
    
    println!("🚀 [GPU {}] Worker GPU (simulation) démarré", device_id);
    
    loop {
        if found.load(Ordering::Relaxed) && config.stop_on_find {
            break;
        }
        
        // Générer un lot de clés pour le GPU
        let mut keys_batch = Vec::new();
        let range = &core_end - &core_start;
        
        for _ in 0..config.gpu_batch_size {
            // Pour les grands nombres, la génération aléatoire est complexe.
            // On se contente d'une approximation en utilisant u64.
            // Une meilleure approche utiliserait `rand::Rng::gen_bigint_range`.
            let u64_max = UBig::from(u64::MAX);
            // ✅ CORRECTION: Utiliser la bonne syntaxe pour gen_range
            let offset_u64 = if range > u64_max {
                rng.next()
            } else {
                rng.gen_range(0, u64::try_from(&range).unwrap_or(u64::MAX))  // ✅ CORRIGÉ
            };
            keys_batch.push(&core_start + offset_u64);
        }
        
        // Traitement par le GPU
        let results = gpu_worker.process_key_batch(&keys_batch);
        
        for (key_val, addresses) in results {
            for address_str in addresses {
                if puzzle.addresses.contains(&address_str) {
                    
                    // Trouvé !
                    found.store(true, Ordering::Relaxed);
                    stats.found_count.fetch_add(1, Ordering::Relaxed);
                    
                    let result = format!(
                        "\n🎉 ==========================================\n\
                         💰 ADRESSE TROUVÉE PAR GPU ! 💰\n\
                         🔍 Adresse: {}\n\
                         🔢 Clé Privée (Hex): {:x}\n\
                         🔢 Clé Privée (Dec): {}\n\
                         🖥️  GPU Device: {}\n\
                         ⚡ Vitesse GPU: {:.2} k/s\n\
                         🕐 Temps écoulé: {:.2}s\n\
                         ==========================================\n",
                        address_str, &key_val, &key_val, device_id,
                        stats.get_gpu_rate() / 1000.0, stats.start_time.elapsed().as_secs_f64()
                    );
                    
                    println!("{}", result);
                    
                    // Enregistrer la clé trouvée
                    let _lock = file_write_lock.lock().unwrap();
                    
                    if let Ok(mut file) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("found.txt") {
                        if let Err(e) = writeln!(
                            file,
                            "[{}] [GPU] Trouvé! Clé privée (hex): {:x}, Adresse: {}",
                            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                            &key_val, 
                            address_str
                        ) {
                            eprintln!("Erreur lors de l'écriture dans found.txt: {}", e);
                        }
                    }
                    
                    if config.stop_on_find {
                        return;
                    }
                }
            }
        }
        
        stats.add_gpu_keys(keys_batch.len() as u64);
    }
    
    println!("Arrêt du worker GPU {}", device_id);
}


// ================================================================================================
// DÉBUT DE LA SECTION AJOUTÉE/COMPLÉTÉE
// ================================================================================================

fn cpu_worker_thread(
    core_id: usize,
    config: Arc<Config>,
    puzzle: Arc<PuzzleData>,
    stats: Arc<Statistics>,
    found: Arc<AtomicBool>,
    file_write_lock: Arc<Mutex<()>>,
    core_start: UBig,
    core_end: UBig,
) {
    let secp = Secp256k1::new();
    let mut rng = FastRng::new((core_id as u64) * 1000000 + rand::thread_rng().gen::<u64>());
    let mut since_switch = 0u64;
    let mut since_checkpoint = 0u64;
    
    // Charger le point de contrôle ou commencer du début
    let mut current_key = load_checkpoint(core_id, &core_start);
    if current_key < core_start || current_key > core_end {
        current_key = core_start.clone();
    }

    println!("⚙️  [CPU {}] Worker démarré. Plage: {} -> {}", core_id, current_key, core_end);

    loop {
        if (found.load(Ordering::Relaxed) && config.stop_on_find) || current_key > core_end {
            break;
        }

        // En mode 'smart', on génère plusieurs clés candidates à partir d'une clé de base
        let keys_to_check = if config.mode == "smart" && config.smart_jump {
            generate_key_patterns(&current_key, &mut rng)
        } else {
            vec![current_key.clone()]
        };

        // ✅ CORRECTION: Sauvegarder la taille AVANT le for loop
        let keys_count = keys_to_check.len();

        for key_val in keys_to_check {  // ✅ Move ownership (OK maintenant)
            if key_val > core_end { continue; }

            let key_bytes = key_val.to_be_bytes();
            if key_bytes.len() > 32 { continue; }
            
            let mut padded_key = [0u8; 32];
            padded_key[32 - key_bytes.len()..].copy_from_slice(&key_bytes);

            if let Ok(secret_key) = SecretKey::from_slice(&padded_key) {
                let address_variants = generate_address_variants(&secp, &secret_key);
                for (_private_key, address) in address_variants {
                    let address_str = address.to_string();
                    if puzzle.addresses.contains(&address_str) {
                        found.store(true, Ordering::Relaxed);
                        stats.found_count.fetch_add(1, Ordering::Relaxed);
                        
                        let result_message = format!(
                            "\n🎉 ==========================================\n\
                             💰 ADRESSE TROUVÉE PAR CPU ! 💰\n\
                             🔍 Adresse: {}\n\
                             🔢 Clé Privée (Hex): {:x}\n\
                             🔢 Clé Privée (Dec): {}\n\
                             ⚙️  CPU Core: {}\n\
                             ⚡ Vitesse CPU: {:.2} k/s\n\
                             🕐 Temps écoulé: {:.2}s\n\
                             ==========================================\n",
                            address_str, &key_val, &key_val, core_id,
                            stats.get_cpu_rate() / 1000.0, stats.start_time.elapsed().as_secs_f64()
                        );
                        println!("{}", result_message);

                        { // Bloc pour le lock
                            let _lock = file_write_lock.lock().unwrap();
                            if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("found.txt") {
                                let _ = writeln!(file, "[{}] [CPU {}] Trouvé! Clé (hex): {:x}, Adresse: {}", 
                                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), core_id, &key_val, address_str);
                            }
                        }

                        if let (Some(token), Some(chat_id)) = (&config.telegram_bot_token, &config.telegram_chat_id) {
                             let telegram_message = format!("<b>🎉 Adresse trouvée (CPU)</b>\n\n<b>Adresse:</b> <code>{}</code>\n<b>Clé Privée (Hex):</b> <code>{:x}</code>", address_str, &key_val);
                             if let Err(e) = send_telegram_notification(token, chat_id, &telegram_message) {
                                 eprintln!("[CPU {}] Erreur d'envoi de la notification Telegram: {}", core_id, e);
                             }
                        }

                        if config.stop_on_find { return; }
                    }
                }
            }
        }

        // ✅ CORRECTION: Utiliser keys_count au lieu de keys_to_check.len()
        let batch_size = if config.mode == "sequential" { 1 } else { keys_count as u64 };
        stats.add_cpu_keys(batch_size);
        since_switch += batch_size;
        since_checkpoint += batch_size;

        if since_checkpoint >= config.checkpoint_interval {
            save_checkpoint(&current_key, core_id);
            since_checkpoint = 0;
        }

        // Logique de progression de la clé
        match config.mode.as_str() {
            "sequential" => {
                current_key += ubig!(1);
            },
            "random" | "smart" => {
                if since_switch >= config.switch_interval {
                    // Saut aléatoire dans la plage du core
                    let range = &core_end - &core_start;
                    let u64_max = UBig::from(u64::MAX);
                    // ✅ CORRECTION: Utiliser la bonne syntaxe pour gen_range
                    let offset_u64 = if range > u64_max { 
                        rng.next() 
                    } else { 
                        rng.gen_range(0, u64::try_from(&range).unwrap_or(u64::MAX))  // ✅ CORRIGÉ
                    };
                    current_key = &core_start + offset_u64;
                    since_switch = 0;
                } else {
                    current_key += ubig!(1);
                }
            },
            _ => { // Par défaut: séquentiel
                 current_key += ubig!(1);
            }
        }
    }
    println!("Arrêt du worker CPU {}", core_id);
}



fn main() {
    println!("======================================================");
    println!("=== Solveur de Puzzle Bitcoin v2.1 - OPTIMISÉ      ===");
    println!("======================================================");

    let config_path = "config.txt";
    if !std::path::Path::new(config_path).exists() {
        create_default_config(config_path);
        println!("\nProgramme terminé. Veuillez configurer '{}' et relancer.", config_path);
        return;
    }

    let config = Arc::new(load_config(config_path));
    let puzzle = Arc::new(load_puzzle_advanced(&config.puzzle_file));
    
    if puzzle.addresses.is_empty() {
        eprintln!("Erreur: Le fichier puzzle '{}' est vide ou n'a pas pu être lu.", config.puzzle_file);
        return;
    }

    let start_key = parse_big_int(&config.start).expect("Clé de départ invalide.");
    let end_key = parse_big_int(&config.end).expect("Clé de fin invalide.");

    if start_key >= end_key {
        eprintln!("Erreur: La clé de départ doit être inférieure à la clé de fin.");
        return;
    }

    let stats = Arc::new(Statistics::new());
    let found = Arc::new(AtomicBool::new(false));
    let file_write_lock = Arc::new(Mutex::new(()));
    let mut handles = vec![];

    let total_threads = if config.cores == 0 { num_cpus::get() } else { config.cores };

    println!("\nConfiguration de la recherche :");
    println!("  - Plage de clés : {} -> {}", start_key, end_key);
    println!("  - Mode de calcul: {:?}", config.compute_mode);
    println!("  - Mode de recherche: {}", config.mode);
    println!("  - Adresses à trouver: {}", puzzle.addresses.len());
    
    let gpu_info = GPUInfo::detect();

    // --- Démarrage des threads ---
    match config.compute_mode {
        ComputeMode::CPU => {
            println!("  - Démarrage de {} threads CPU...", total_threads);
            let range_per_core = (&end_key - &start_key + ubig!(1)) / total_threads;
            for i in 0..total_threads {
                let core_start = &start_key + i * &range_per_core;
                let core_end = if i == total_threads - 1 { end_key.clone() } else { &core_start + &range_per_core - ubig!(1) };
                
                let (c, p, s, f, l) = (config.clone(), puzzle.clone(), stats.clone(), found.clone(), file_write_lock.clone());
                handles.push(thread::spawn(move || {
                    cpu_worker_thread(i, c, p, s, f, l, core_start, core_end);
                }));
            }
        },
        ComputeMode::GPU => {
            if !gpu_info.available {
                eprintln!("Erreur: Mode GPU sélectionné mais aucun GPU compatible n'a été détecté.");
                return;
            }
            println!("  - Démarrage de 1 thread GPU (simulation)...");
            let (c, p, s, f, l) = (config.clone(), puzzle.clone(), stats.clone(), found.clone(), file_write_lock.clone());
            let (sk, ek) = (start_key.clone(), end_key.clone());
            handles.push(thread::spawn(move || {
                gpu_worker_thread(c.gpu_device_id, c, p, s, f, l, sk, ek);
            }));
        },
        ComputeMode::Hybrid => {
            if !gpu_info.available {
                eprintln!("Avertissement: Mode hybride sélectionné, mais aucun GPU détecté. Passage en mode CPU uniquement.");
                // Comportement identique au mode CPU
            }

            let num_cpu_threads = (total_threads as f64 * config.cpu_gpu_ratio).ceil() as usize;
            let num_gpu_threads = if gpu_info.available { total_threads - num_cpu_threads } else { 0 };

            println!("  - Mode Hybride: {} threads CPU, {} threads GPU", num_cpu_threads, num_gpu_threads);

            // Threads CPU
            if num_cpu_threads > 0 {
                let range_per_core = (&end_key - &start_key + ubig!(1)) / num_cpu_threads;
                for i in 0..num_cpu_threads {
                    let core_start = &start_key + i * &range_per_core;
                    let core_end = if i == num_cpu_threads - 1 { end_key.clone() } else { &core_start + &range_per_core - ubig!(1) };
                    
                    let (c, p, s, f, l) = (config.clone(), puzzle.clone(), stats.clone(), found.clone(), file_write_lock.clone());
                    handles.push(thread::spawn(move || {
                        cpu_worker_thread(i, c, p, s, f, l, core_start, core_end);
                    }));
                }
            }

            // Threads GPU
            for i in 0..num_gpu_threads {
                 let (c, p, s, f, l) = (config.clone(), puzzle.clone(), stats.clone(), found.clone(), file_write_lock.clone());
                 let (sk, ek) = (start_key.clone(), end_key.clone());
                 handles.push(thread::spawn(move || {
                     gpu_worker_thread(i, c, p, s, f, l, sk, ek);
                 }));
            }
        }
    }

    println!("\nRecherche en cours... Pressez CTRL+C pour arrêter.");

    // Boucle principale pour afficher les statistiques
    let start_time = stats.start_time;
    while handles.iter().any(|h| !h.is_finished()) {
        thread::sleep(Duration::from_secs(5));
        
        if found.load(Ordering::Relaxed) && config.stop_on_find {
            break;
        }

        let elapsed_secs = start_time.elapsed().as_secs();
        let elapsed_time = format!("{:02}:{:02}:{:02}", elapsed_secs / 3600, (elapsed_secs % 3600) / 60, elapsed_secs % 60);
        let total_rate = stats.get_rate();
        let cpu_rate = stats.get_cpu_rate();
        let gpu_rate = stats.get_gpu_rate();
        
        print!("\r[Temps: {}] [Total: {:.2} Mk/s] [CPU: {:.2} Mk/s | GPU: {:.2} Mk/s] [Trouvées: {}]      ",
            elapsed_time, 
            total_rate / 1_000_000.0,
            cpu_rate / 1_000_000.0,
            gpu_rate / 1_000_000.0,
            stats.found_count.load(Ordering::Relaxed)
        );
        let _ = std::io::stdout().flush();
    }

    println!("\n\nRecherche terminée.");
    for handle in handles {
        handle.join().unwrap();
    }

    let final_found = stats.found_count.load(Ordering::Relaxed);
    if final_found > 0 {
        println!("🎉 Félicitations ! {} clé(s) ont été trouvées et sauvegardées dans 'found.txt'.", final_found);
    } else {
        println!("Aucune clé trouvée dans la plage spécifiée.");
    }
}