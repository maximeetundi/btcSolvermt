#!/bin/bash

# Script de compilation optimisé pour le solveur de puzzle Bitcoin
# Utilisation: ./build.sh [release|debug|benchmark]

set -e

echo "🚀 Bitcoin Puzzle Solver - Script de Build Optimisé"
echo "=================================================="

# Couleurs pour les messages
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Fonction pour afficher les messages colorés
print_status() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

print_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

print_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Vérifier si Rust est installé
if ! command -v rustc &> /dev/null; then
    print_error "Rust n'est pas installé. Veuillez installer Rust depuis https://rustup.rs/"
    exit 1
fi

# Vérifier si Cargo est installé
if ! command -v cargo &> /dev/null; then
    print_error "Cargo n'est pas installé. Veuillez réinstaller Rust."
    exit 1
fi

print_status "Version de Rust: $(rustc --version)"
print_status "Version de Cargo: $(cargo --version)"

# Nettoyer les anciens builds
print_status "Nettoyage des anciens builds..."
cargo clean

# Mettre à jour les dépendances
print_status "Mise à jour des dépendances..."
cargo update

# Définir le mode de build
BUILD_MODE=${1:-release}

case $BUILD_MODE in
    "release")
        print_status "Compilation en mode RELEASE (optimisé)..."
        
        # Variables d'environnement pour optimiser
        export RUSTFLAGS="-C target-cpu=native -C target-feature=+crt-static"
        export CARGO_PROFILE_RELEASE_LTO=true
        export CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1
        export CARGO_PROFILE_RELEASE_PANIC=abort
        
        # Compilation optimisée
        cargo build --release
        
        # Vérifier si la compilation a réussi
        if [ $? -eq 0 ]; then
            print_success "Compilation réussie !"
            print_status "Exécutable généré: ./target/release/bitcoin_puzzle_solver"
            
            # Afficher la taille de l'exécutable
            if [ -f "./target/release/bitcoin_puzzle_solver" ]; then
                SIZE=$(du -h "./target/release/bitcoin_puzzle_solver" | cut -f1)
                print_status "Taille de l'exécutable: $SIZE"
            fi
        else
            print_error "Échec de la compilation"
            exit 1
        fi
        ;;
        
    "debug")
        print_status "Compilation en mode DEBUG..."
        cargo build
        
        if [ $? -eq 0 ]; then
            print_success "Compilation debug réussie !"
            print_status "Exécutable généré: ./target/debug/bitcoin_puzzle_solver"
        else
            print_error "Échec de la compilation debug"
            exit 1
        fi
        ;;
        
    "benchmark")
        print_status "Compilation et exécution des benchmarks..."
        
        # Vérifier si le feature benchmark est disponible
        if grep -q "benchmark" Cargo.toml; then
            cargo bench --features benchmark
            
            if [ $? -eq 0 ]; then
                print_success "Benchmarks terminés avec succès !"
            else
                print_error "Échec des benchmarks"
                exit 1
            fi
        else
            print_warning "Feature benchmark non disponible"
            print_status "Compilation normale en mode release..."
            cargo build --release
        fi
        ;;
        
    "test")
        print_status "Exécution des tests..."
        cargo test
        
        if [ $? -eq 0 ]; then
            print_success "Tous les tests sont passés !"
        else
            print_error "Certains tests ont échoué"
            exit 1
        fi
        ;;
        
    *)
        print_error "Mode de build non reconnu: $BUILD_MODE"
        echo "Modes disponibles: release, debug, benchmark, test"
        exit 1
        ;;
esac

# Créer le fichier de configuration par défaut s'il n'existe pas
if [ ! -f "config.txt" ]; then
    print_status "Création du fichier de configuration par défaut..."
    
    # Détecter le nombre de CPUs
    if command -v nproc &> /dev/null; then
        CPU_COUNT=$(nproc)
    elif [ -f /proc/cpuinfo ]; then
        CPU_COUNT=$(grep -c ^processor /proc/cpuinfo)
    else
        CPU_COUNT=4
    fi
    
    cat > config.txt << EOF
# Configuration Bitcoin Puzzle Solver - Générée automatiquement
# Détection automatique: $CPU_COUNT CPUs disponibles

# Plage de recherche pour puzzle 66 (exemple)
start=0x20000000000000000
end=0x3ffffffffffffffff

# Nombre de coeurs CPU (0 = détection automatique)
cores=0

# Mode de recherche optimisé
mode=smart

# Intervalle de commutation (plus élevé = plus efficace)
switch_interval=2000000

# Ratio de sous-intervalle (plus petit = plus précis)
subinterval_ratio=0.0001

# Arrêt automatique à la découverte
stop_on_find=true

# Fichier des adresses cibles
puzzle_file=puzzle.txt

# Algorithmes avancés
baby_steps=true
giant_steps=true
bloom_filter=false
smart_jump=true

# Optimisations de performance
batch_size=50000
checkpoint_interval=50000000
EOF
    
    print_success "Fichier config.txt créé avec $CPU_COUNT CPUs détectés"
fi

# Créer un exemple de fichier puzzle s'il n'existe pas
if [ ! -f "puzzle.txt" ]; then
    print_status "Création d'un fichier puzzle.txt d'exemple..."
    
    cat > puzzle.txt << EOF
# Exemple d'adresses Bitcoin pour les puzzles
# Remplacez par les vraies adresses que vous voulez résoudre

# Puzzle 1 (résolu - pour test)
1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH

# Puzzle 66 (non résolu)
13zb1hQbWVsc2S7ZTZnP2G4undNNpdh5so

# Ajoutez d'autres adresses ici...
EOF
    
    print_success "Fichier puzzle.txt d'exemple créé"
fi

print_status "Création des scripts d'aide..."

# Créer un script de lancement rapide
cat > run.sh << 'EOF'
#!/bin/bash
echo "🚀 Lancement du Bitcoin Puzzle Solver..."

# Vérifier si l'exécutable existe
if [ ! -f "./target/release/bitcoin_puzzle_solver" ]; then
    echo "⚠️  Exécutable non trouvé. Compilation en cours..."
    ./build.sh release
fi

# Lancer le solveur
echo "🔍 Démarrage de la recherche..."
./target/release/bitcoin_puzzle_solver

echo "✅ Recherche terminée."
EOF

chmod +x run.sh

# Créer un script de nettoyage
cat > clean.sh << 'EOF'
#!/bin/bash
echo "🧹 Nettoyage des fichiers temporaires..."

# Supprimer les fichiers de build
cargo clean

# Supprimer les checkpoints
rm -f checkpoint_core_*.txt

# Supprimer les logs
rm -f solver.log

echo "✅ Nettoyage terminé."
EOF

chmod +x clean.sh

print_success "Scripts d'aide créés: run.sh, clean.sh"

echo ""
echo "🎉 Build terminé avec succès !"
echo "📋 Fichiers générés:"
echo "   - Exécutable: ./target/$BUILD_MODE/bitcoin_puzzle_solver"
echo "   - Configuration: ./config.txt"
echo "   - Puzzle exemple: ./puzzle.txt"
echo "   - Script de lancement: ./run.sh"
echo "   - Script de nettoyage: ./clean.sh"
echo ""
echo "🚀 Pour démarrer la recherche:"
echo "   ./run.sh"
echo ""
echo "⚙️  Pour modifier la configuration:"
echo "   nano config.txt"
echo ""
echo "📊 Pour exécuter les benchmarks:"
echo "   ./build.sh benchmark"