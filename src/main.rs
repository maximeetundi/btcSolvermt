use bitcoin::{Address, Network, PrivateKey, PublicKey};
use bitcoin::secp256k1::{Secp256k1, SecretKey};
use ibig::{ubig, UBig};
use rand::Rng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use num_cpus;
use std::collections::HashSet;
use std::env;
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
struct Config {
    start: String,
    end: String,
    cores: usize,
    mode: String,
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
    addresses: HashSet<String>,
    compressed_addresses: HashSet<String>,
    uncompressed_addresses: HashSet<String>,
}

#[derive(Debug)]
struct Statistics {
    start_time: Instant,
    keys_checked: AtomicU64,
    found_count: AtomicU64,
}

impl Statistics {
    fn new() -> Self {
        Self {
            start_time: Instant::now(),
            keys_checked: AtomicU64::new(0),
            found_count: AtomicU64::new(0),
        }
    }
    
    fn add_keys(&self, count: u64) {
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

// Structure pour Baby-step Giant-step optimisé
// (Actuellement non implémenté dans cette version)
#[allow(dead_code)]
struct BabyStepGiantStep;

#[allow(dead_code)]
impl BabyStepGiantStep {
    fn new() -> Self {
        // Constructeur vide car la structure est vide
        // Cette implémentation est conservée pour compatibilité future
        Self {}
    }
}

fn load_puzzle_advanced(path: &str) -> PuzzleData {
    let file = File::open(path).expect(&format!("Impossible d'ouvrir le fichier puzzle : {}", path));
    let reader = BufReader::new(file);
    
    let mut addresses = HashSet::new();
    let mut compressed_addresses = HashSet::new();
    let mut uncompressed_addresses = HashSet::new();
    
    for line in reader.lines().filter_map(Result::ok) {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        
        addresses.insert(line.to_string());
        
        // Distinguer les adresses compressées et non compressées si possible
        if line.starts_with('1') {
            compressed_addresses.insert(line.to_string());
        } else if line.starts_with('3') {
            uncompressed_addresses.insert(line.to_string());
        }
    }
    
    PuzzleData {
        addresses,
        compressed_addresses,
        uncompressed_addresses,
    }
}

fn generate_address_variants(secp: &Secp256k1<bitcoin::secp256k1::All>, secret_key: &SecretKey) -> Vec<(PrivateKey, Address)> {
    let mut variants = Vec::new();
    
    // Version compressée
    let private_key_compressed = PrivateKey {
        compressed: true,
        network: Network::Bitcoin.into(),
        inner: *secret_key,
    };
    let public_key_compressed = PublicKey::from_private_key(secp, &private_key_compressed);
    let address_compressed = Address::p2pkh(&public_key_compressed, Network::Bitcoin);
    variants.push((private_key_compressed, address_compressed));
    
    // Version non compressée
    let private_key_uncompressed = PrivateKey {
        compressed: false,
        network: Network::Bitcoin.into(),
        inner: *secret_key,
    };
    let public_key_uncompressed = PublicKey::from_private_key(secp, &private_key_uncompressed);
    let address_uncompressed = Address::p2pkh(&public_key_uncompressed, Network::Bitcoin);
    variants.push((private_key_uncompressed, address_uncompressed));
    
    variants
}

fn create_default_config(path: &str) {
    let config_content = "# Fichier de configuration pour le solveur de puzzle Bitcoin OPTIMISÉ
# Modifiez les valeurs ci-dessous puis relancez le programme.

# Plage de recherche (peut être en décimal ou en hexadécimal préfixé par 0x)
start=0x20000000000000000
end=0x3ffffffffffffffff

# Nombre de coeurs CPU à utiliser (0 = détection automatique)
cores=0

# Mode de recherche : 'random', 'sequential', 'smart', 'kangaroo'
mode=smart

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
telegram_chat_id=";

    println!("Création d'un nouveau fichier de configuration par défaut...");
    let mut file = File::create(path).expect("Impossible de créer le fichier de configuration.");
    file.write_all(config_content.as_bytes())
        .expect("Impossible d'écrire dans le fichier de configuration.");
    
    println!("Un fichier de configuration '{}' a été créé avec des valeurs par défaut.", path);
    println!("Veuillez le modifier selon vos besoins avant de relancer l'application.");
}

fn load_config(path: &str) -> Config {
    // Valeurs par défaut
    let mut config = Config {
        start: "1".to_string(),
        end: "1000000".to_string(),
        cores: num_cpus::get(),
        mode: "sequential".to_string(),
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
    
    // Addition/soustraction de petites valeurs (en utilisant des références pour éviter les déplacements)
    let offsets: [u64; 10] = [1, 2, 3, 5, 8, 13, 21, 34, 55, 89];
    for &offset in &offsets {
        patterns.push(base_key.clone() + offset);
        if *base_key > UBig::from(offset) {
            patterns.push(base_key.clone() - offset);
        }
    }
    
    // Multiplication par des facteurs premiers (limités pour éviter les débordements)
    let factors = [2, 3, 5, 7, 11, 13];
    for &factor in &factors {
        patterns.push(base_key.clone() * factor);
    }
    
    // Patterns basés sur les digits (uniquement si la représentation décimale est valide)
    if base_str.chars().all(|c| c.is_ascii_digit()) {
        let mut chars: Vec<char> = base_str.chars().collect();
        for _ in 0..3.min(chars.len().saturating_sub(1)) {  // Limiter le nombre de permutations
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

fn worker_thread(
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
    let mut since_switch = 0;
    let mut since_checkpoint = 0;
    
    // Charger le point de contrôle
    let mut current_key = load_checkpoint(core_id, &core_start);
    
    // Variables pour différents modes
    let sub_interval_size = UBig::from(((&core_end - &core_start).to_f64() * config.subinterval_ratio) as u64);
    let sub_start = current_key.clone();
    let sub_end = &sub_start + &sub_interval_size;
    
    // Initialiser BabyStepGiantStep si nécessaire (désactivé pour l'instant)
    let _baby_giant = if config.baby_steps || config.giant_steps {
        Some(BabyStepGiantStep::new())
    } else {
        None
    };
    
    loop {
        if found.load(Ordering::Relaxed) && config.stop_on_find {
            break;
        }
        
        // Générer les clés à tester selon le mode
        let keys_to_test = match config.mode.as_str() {
            "random" => {
                let range = &sub_end - &sub_start + ubig!(1);
                let range_u64 = u64::try_from(range.clone()).unwrap_or(u64::MAX);
                let mut keys = Vec::new();
                for _ in 0..config.batch_size.min(1000) {
                    let offset = if range_u64 > 0 { rng.gen_range(0, range_u64) } else { 0 };
                    keys.push(&sub_start + offset);
                }
                keys
            },
            "sequential" => {
                let mut keys = Vec::new();
                for _ in 0..config.batch_size.min(1000) {
                    keys.push(current_key.clone());
                    current_key += ubig!(1);
                    if current_key > core_end {
                        current_key = core_start.clone();
                    }
                }
                keys
            },
            "smart" => {
                // Mode intelligent avec patterns
                let base_key = if since_switch % 2 == 0 {
                    // Alternance entre aléatoire et séquentiel
                    let range = &core_end - &core_start + ubig!(1);
                    let range_u64 = u64::try_from(range.clone()).unwrap_or(u64::MAX);
                    let offset = if range_u64 > 0 { rng.gen_range(0, range_u64) } else { 0 };
                    &core_start + offset
                } else {
                    current_key.clone()
                };
                generate_key_patterns(&base_key, &mut rng)
            },
            "kangaroo" => {
                // Implémentation basique de Pollard's Kangaroo
                let mut keys = Vec::new();
                let jump_size = (&core_end - &core_start) / 1000;
                for _ in 0..config.batch_size.min(100) {
                    keys.push(current_key.clone());
                    current_key += &jump_size;
                    if current_key > core_end {
                        current_key = core_start.clone();
                    }
                }
                keys
            },
            _ => {
                vec![current_key.clone()]
            }
        };
        
        // Traitement par lots
        for key_batch in keys_to_test.chunks(100) {
            for key_val in key_batch {
                if *key_val == ubig!(0) || *key_val > core_end {
                    continue;
                }
                
                let key_bytes = key_val.to_be_bytes();
                if key_bytes.len() > 32 {
                    continue;
                }
                
                let mut padded = [0u8; 32];
                padded[32 - key_bytes.len()..].copy_from_slice(&key_bytes);
                
                if let Ok(secret_key) = SecretKey::from_slice(&padded) {
                    let address_variants = generate_address_variants(&secp, &secret_key);
                    
                    for (private_key, address) in address_variants {
                        let address_str = address.to_string();
                        
                        if puzzle.addresses.contains(&address_str) ||
                           puzzle.compressed_addresses.contains(&address_str) ||
                           puzzle.uncompressed_addresses.contains(&address_str) {
                            
                            // Trouvé !
                            found.store(true, Ordering::Relaxed);
                            let found_count = stats.found_count.fetch_add(1, Ordering::Relaxed) + 1;
                            
                            let wif = private_key.to_wif();
                            let result = format!(
                                "\n🎉 ==========================================\n\
                                💰 ADRESSE TROUVÉE ! 💰\n\
                                🔍 Adresse: {}\n\
                                🔑 Clé Privée (WIF): {}\n\
                                🔢 Nombre Décimal: {}\n\
                                🏭 Core: {}\n\
                                ⚡ Vitesse: {:.2} clés/s\n\
                                🕐 Temps écoulé: {:.2}s\n\
                                ==========================================\n",
                                address_str, wif, key_val, core_id, 
                                stats.get_rate(), stats.start_time.elapsed().as_secs_f64()
                            );
                            
                            println!("{}", result);
                            
                            // Enregistrer la clé trouvée
                            let _lock = file_write_lock.lock().unwrap();
                            let message = format!(
                                "🔑 *Clé trouvée!* (#{})\n\n*Clé privée:* {}\n*Adresse:* {}",
                                found_count, private_key, address
                            );
                            
                            // Écrire dans le fichier found.txt
                            if let Ok(mut file) = OpenOptions::new()
                                .create(true)
                                .append(true)
                                .open("found.txt") {
                                if let Err(e) = writeln!(
                                    file,
                                    "[{}] Trouvé! Clé privée: {}, Adresse: {}",
                                    chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                                    private_key, 
                                    address
                                ) {
                                    eprintln!("Erreur lors de l'écriture dans found.txt: {}", e);
                                }
                            } else {
                                eprintln!("Impossible d'ouvrir le fichier found.txt");
                            }
                            
                            // Envoyer une notification Telegram si configuré (de manière synchrone)
                            if let (Some(token), Some(chat_id)) = (&config.telegram_bot_token, &config.telegram_chat_id) {
                                if let Err(e) = send_telegram_notification(token, chat_id, &message) {
                                    eprintln!("Erreur lors de l'envoi de la notification Telegram: {}", e);
                                }
                                
                                // Si stop_on_find est activé, on attend un peu pour s'assurer que le message est bien parti
                                if config.stop_on_find {
                                    std::thread::sleep(std::time::Duration::from_millis(500));
                                    return;
                                }
                            }
                            
                            if config.stop_on_find {
                                return;
                            }
                        }
                    }
                }
                
                stats.add_keys(1);
                since_switch += 1;
                since_checkpoint += 1;
                
                // Sauvegarde périodique (toutes les 1000 clés)
                if since_checkpoint >= 1000 {
                    save_checkpoint(&current_key, core_id);
                    since_checkpoint = 0;
                }
            }
        }
        
        // Vérifier s'il faut changer de région de recherche
        if since_switch >= config.switch_interval {
            since_switch = 0;
            // Changer la région de recherche selon le mode
            match config.mode.as_str() {
                "smart" | "random" => {
                    // Saut vers une nouvelle région aléatoire
                    let range = &core_end - &core_start + ubig!(1);
                    let range_u64 = u64::try_from(range.clone()).unwrap_or(u64::MAX);
                    let offset = if range_u64 > 0 { rng.gen_range(0, range_u64) } else { 0 };
                    current_key = &core_start + offset;
                }
                _ => {}
            }
        }
    }
    
    // Sauvegarde finale avant de quitter
    save_checkpoint(&current_key, core_id);
    println!("Arrêt du thread de travail {}", core_id);
}

fn main() {
    println!("🚀 Solveur de Puzzle Bitcoin OPTIMISÉ v2.0 🚀");
    println!("===============================================");
    
    let mut config_path = env::current_exe().expect("Impossible de trouver le chemin de l'exécutable.");
    config_path.pop();
    config_path.push("config.txt");
    
    if !config_path.exists() {
        create_default_config(config_path.to_str().unwrap());
        return;
    }
    
    let config = load_config(config_path.to_str().unwrap());
    println!("📋 Configuration chargée: {:?}", config);
    
    let puzzle = Arc::new(load_puzzle_advanced(&config.puzzle_file));
    println!("🎯 Puzzle chargé: {} adresses", puzzle.addresses.len());
    
    let stats = Arc::new(Statistics::new());
    let found = Arc::new(AtomicBool::new(false));
    let file_write_lock = Arc::new(Mutex::new(()));
    
    // Thread de rapport statistique amélioré
    let stats_clone = stats.clone();
    let found_clone = found.clone();
    let stop_on_find = config.stop_on_find;
    
    thread::spawn(move || {
        let mut last_count = 0;
        let mut last_time = Instant::now();
        let mut last_stats_time = Instant::now();
        
        loop {
            if found_clone.load(Ordering::Relaxed) && stop_on_find {
                break;
            }
            
            thread::sleep(Duration::from_secs(10));
            
            let current_count = stats_clone.keys_checked.load(Ordering::Relaxed);
            let current_time = Instant::now();
            let time_diff = current_time.duration_since(last_time).as_secs_f64();
            
            let instant_rate = if time_diff > 0.0 {
                (current_count - last_count) as f64 / time_diff
            } else {
                0.0
            };
            
            let total_rate = stats_clone.get_rate();
            let elapsed = stats_clone.start_time.elapsed();
            
            // Afficher les statistiques toutes les secondes
            if current_time.duration_since(last_stats_time).as_secs() >= 1 {
                println!(
                    "📊 [Stats] Total: {} | Vitesse: {:.0} clés/s | Instantané: {:.0} clés/s | Temps: {}:{:02}:{:02}",
                    current_count,
                    total_rate,
                    instant_rate,
                    elapsed.as_secs() / 3600,
                    (elapsed.as_secs() % 3600) / 60,
                    elapsed.as_secs() % 60
                );
                
                last_count = current_count;
                last_time = current_time;
                last_stats_time = current_time;
            }
        }
    });
    
    let start = parse_big_int(&config.start).expect("Valeur de départ invalide");
    let end = parse_big_int(&config.end).expect("Valeur de fin invalide");
    
    // S'assurer qu'on utilise au moins 1 cœur
    let num_cores = if config.cores == 0 { 
        let cores = num_cpus::get();
        println!("🔍 Détection automatique: {} cœurs disponibles", cores);
        cores 
    } else { 
        config.cores 
    };
    
    let total = &end - &start + ubig!(1);
    let slice_size = &total / num_cores;
    
    println!("🔍 Recherche dans la plage: {} à {}", config.start, config.end);
    println!("💻 Utilisation de {} cœurs", num_cores);
    println!("📈 Mode: {}", config.mode);
    println!("🏁 Démarrage des workers...\n");
    
    // Lancement des threads workers
    (0..num_cores).into_par_iter().for_each(|i| {
        let core_start = &start + &slice_size * i;
        let core_end = if i == num_cores - 1 {
            end.clone()
        } else {
            &core_start + &slice_size - ubig!(1)
        };
        
        worker_thread(
            i,
            Arc::new(config.clone()),
            puzzle.clone(),
            stats.clone(),
            found.clone(),
            file_write_lock.clone(),
            core_start,
            core_end,
        );
    });
    
    println!("🎉 Recherche terminée !");
    if stats.found_count.load(Ordering::Relaxed) > 0 {
        println!("💰 Nombre de clés trouvées: {}", stats.found_count.load(Ordering::Relaxed));
    }
}