#!/bin/bash

# Script de démarrage pour le Bitcoin Solver avec monitoring
# Usage: ./start_monitor.sh [start|stop|restart|status]

SCRIPT_NAME="bitcoin_puzzle_solver"
LOG_FILE="bitcoin_solver.log"
PID_FILE="bitcoin_solver.pid"
SCRIPT_PATH="./$SCRIPT_NAME"

# Couleurs pour l'affichage
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Fonction pour afficher les messages
log_message() {
    echo -e "${BLUE}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ✅ $1"
}

log_error() {
    echo -e "${RED}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ❌ $1"
}

log_warning() {
    echo -e "${YELLOW}[$(date '+%Y-%m-%d %H:%M:%S')]${NC} ⚠️ $1"
}

# Fonction pour vérifier si le processus est en cours
is_running() {
    if [ -f "$PID_FILE" ]; then
        local pid=$(cat "$PID_FILE")
        if ps -p $pid > /dev/null 2>&1; then
            return 0
        else
            # PID file existe mais processus mort, nettoyer
            rm -f "$PID_FILE"
            return 1
        fi
    fi
    return 1
}

# Fonction pour obtenir le PID
get_pid() {
    if [ -f "$PID_FILE" ]; then
        cat "$PID_FILE"
    else
        echo ""
    fi
}

# Fonction pour démarrer le script
start_solver() {
    log_message "Démarrage du Bitcoin Solver..."
    
    # Vérifier si déjà en cours
    if is_running; then
        log_warning "Le Bitcoin Solver est déjà en cours d'exécution (PID: $(get_pid))"
        return 1
    fi
    
    # Vérifier que l'exécutable existe
    if [ ! -f "$SCRIPT_PATH" ]; then
        log_error "L'exécutable $SCRIPT_PATH n'existe pas!"
        log_message "Compilez d'abord avec: cargo build --release"
        return 1
    fi
    
    # Vérifier que l'exécutable est... exécutable
    if [ ! -x "$SCRIPT_PATH" ]; then
        log_warning "L'exécutable n'a pas les permissions d'exécution"
        chmod +x "$SCRIPT_PATH"
        log_success "Permissions d'exécution ajoutées"
    fi
    
    # Sauvegarder l'ancien log si il existe
    if [ -f "$LOG_FILE" ]; then
        local backup_file="${LOG_FILE}.$(date +%Y%m%d_%H%M%S).bak"
        mv "$LOG_FILE" "$backup_file"
        log_message "Ancien log sauvegardé vers $backup_file"
    fi
    
    # Démarrer le processus en arrière-plan
    log_message "Lancement de $SCRIPT_PATH..."
    nohup "$SCRIPT_PATH" > "$LOG_FILE" 2>&1 &
    local pid=$!
    
    # Sauvegarder le PID
    echo $pid > "$PID_FILE"
    
    # Attendre un peu pour vérifier que le processus démarre correctement
    sleep 2
    
    if is_running; then
        log_success "Bitcoin Solver démarré avec succès (PID: $pid)"
        log_message "Logs disponibles dans: $LOG_FILE"
        log_message "Interface web: Ouvrez bitcoin_monitor_interface.html dans votre navigateur"
        return 0
    else
        log_error "Échec du démarrage du Bitcoin Solver"
        if [ -f "$LOG_FILE" ]; then
            log_message "Dernières lignes du log:"
            tail -n 10 "$LOG_FILE"
        fi
        return 1
    fi
}

# Fonction pour arrêter le script
stop_solver() {
    log_message "Arrêt du Bitcoin Solver..."
    
    if ! is_running; then
        log_warning "Le Bitcoin Solver n'est pas en cours d'exécution"
        return 1
    fi
    
    local pid=$(get_pid)
    log_message "Arrêt du processus PID: $pid"
    
    # Essayer d'arrêter proprement avec SIGTERM
    kill -TERM $pid 2>/dev/null
    
    # Attendre jusqu'à 10 secondes
    local count=0
    while [ $count -lt 10 ] && is_running; do
        sleep 1
        count=$((count + 1))
        echo -n "."
    done
    echo ""
    
    if is_running; then
        log_warning "Arrêt forcé du processus..."
        kill -KILL $pid 2>/dev/null
        sleep 1
    fi
    
    if ! is_running; then
        rm -f "$PID_FILE"
        log_success "Bitcoin Solver arrêté avec succès"
        return 0
    else
        log_error "Impossible d'arrêter le processus"
        return 1
    fi
}

# Fonction pour redémarrer
restart_solver() {
    log_message "Redémarrage du Bitcoin Solver..."
    stop_solver
    sleep 2
    start_solver
}

# Fonction pour afficher le statut
show_status() {
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
    echo "               🚀 BITCOIN SOLVER MONITOR 🚀"
    echo "═══════════════════════════════════════════════════════════════"
    echo ""
    
    if is_running; then
        local pid=$(get_pid)
        log_success "Statut: EN COURS D'EXÉCUTION"
        echo "  📍 PID: $pid"
        
        # Informations système
        if command -v ps >/dev/null 2>&1; then
            local cpu=$(ps -p $pid -o %cpu --no-headers 2>/dev/null | tr -d ' ')
            local mem=$(ps -p $pid -o %mem --no-headers 2>/dev/null | tr -d ' ')
            local rss=$(ps -p $pid -o rss --no-headers 2>/dev/null | tr -d ' ')
            local etime=$(ps -p $pid -o etime --no-headers 2>/dev/null | tr -d ' ')
            
            echo "  💻 CPU: ${cpu}%"
            echo "  🧠 Mémoire: ${mem}% ($(echo "$rss/1024" | bc 2>/dev/null || echo "N/A")MB)"
            echo "  ⏱️  Temps: $etime"
        fi
        
        # Statistiques du log
        if [ -f "$LOG_FILE" ]; then
            echo "  📄 Log: $LOG_FILE ($(du -h "$LOG_FILE" | cut -f1))"
            
            # Dernière ligne de stats si disponible
            local last_stats=$(grep -E "(Total:|Vitesse:|clés/s)" "$LOG_FILE" | tail -1)
            if [ ! -z "$last_stats" ]; then
                echo "  📊 $last_stats"
            fi
        fi
        
    else
        log_error "Statut: ARRÊTÉ"
        if [ -f "$PID_FILE" ]; then
            log_warning "Fichier PID orphelin détecté, nettoyage..."
            rm -f "$PID_FILE"
        fi
    fi
    
    echo ""
    echo "  📁 Fichiers importants:"
    echo "    • Exécutable: $SCRIPT_PATH"
    echo "    • Log: $LOG_FILE"
    echo "    • PID: $PID_FILE"
    echo "    • Config: config.txt"
    echo "    • Found: found.txt"
    echo ""
    echo "  🌐 Interface web: bitcoin_monitor_interface.html"
    echo "  📱 API: bitcoin_monitor.php"
    echo ""
    echo "═══════════════════════════════════════════════════════════════"
}

# Fonction pour surveiller en temps réel
monitor_live() {
    if ! is_running; then
        log_error "Le Bitcoin Solver n'est pas en cours d'exécution"
        return 1
    fi
    
    log_message "Surveillance en temps réel (Ctrl+C pour arrêter)..."
    echo ""
    
    # Suivre le log en temps réel
    if [ -f "$LOG_FILE" ]; then
        tail -f "$LOG_FILE"
    else
        log_error "Fichier de log non trouvé: $LOG_FILE"
        return 1
    fi
}

# Fonction pour compiler le projet
build_project() {
    log_message "Compilation du projet Bitcoin Solver..."
    
    if [ ! -f "Cargo.toml" ]; then
        log_error "Fichier Cargo.toml non trouvé. Êtes-vous dans le bon répertoire ?"
        return 1
    fi
    
    # Compiler en mode release
    cargo build --release
    
    if [ $? -eq 0 ]; then
        log_success "Compilation réussie!"
        log_message "Exécutable créé: $SCRIPT_PATH"
    else
        log_error "Échec de la compilation"
        return 1
    fi
}

# Fonction pour nettoyer les fichiers
cleanup() {
    log_message "Nettoyage des fichiers temporaires..."
    
    # Arrêter le processus si en cours
    if is_running; then
        log_warning "Arrêt du processus en cours..."
        stop_solver
    fi
    
    # Nettoyer les fichiers
    rm -f "$PID_FILE"
    
    if [ "$1" = "all" ]; then
        rm -f "$LOG_FILE"
        rm -f "found.txt"
        rm -f *.log.*.bak
        log_success "Tous les fichiers nettoyés"
    else
        log_success "Fichiers temporaires nettoyés"
    fi
}

# Fonction d'aide
show_help() {
    echo ""
    echo "🚀 Bitcoin Solver Monitor - Script de gestion"
    echo ""
    echo "Usage: $0 [COMMANDE]"
    echo ""
    echo "Commandes disponibles:"
    echo "  start      Démarrer le Bitcoin Solver"
    echo "  stop       Arrêter le Bitcoin Solver"
    echo "  restart    Redémarrer le Bitcoin Solver"
    echo "  status     Afficher le statut détaillé"
    echo "  monitor    Surveillance en temps réel des logs"
    echo "  build      Compiler le projet"
    echo "  cleanup    Nettoyer les fichiers temporaires"
    echo "  cleanup-all Nettoyer tous les fichiers (logs inclus)"
    echo "  help       Afficher cette aide"
    echo ""
    echo "Exemples:"
    echo "  $0 start          # Démarrer le solver"
    echo "  $0 status         # Voir le statut"
    echo "  $0 monitor        # Surveiller en temps réel"
    echo ""
}

# Script principal
main() {
    local command=${1:-status}
    
    case "$command" in
        "start")
            start_solver
            ;;
        "stop")
            stop_solver
            ;;
        "restart")
            restart_solver
            ;;
        "status")
            show_status
            ;;
        "monitor")
            monitor_live
            ;;
        "build")
            build_project
            ;;
        "cleanup")
            cleanup
            ;;
        "cleanup-all")
            cleanup all
            ;;
        "help"|"-h"|"--help")
            show_help
            ;;
        *)
            log_error "Commande inconnue: $command"
            show_help
            exit 1
            ;;
    esac
}

# Vérification des prérequis
check_requirements() {
    # Vérifier que nous sommes dans un répertoire avec un projet Rust
    if [ ! -f "Cargo.toml" ]; then
        log_warning "Cargo.toml non trouvé. Assurez-vous d'être dans le répertoire du projet."
    fi
    
    # Vérifier que les utilitaires nécessaires sont disponibles
    for cmd in ps kill; do
        if ! command -v $cmd >/dev/null 2>&1; then
            log_error "Commande requise non trouvée: $cmd"
            exit 1
        fi
    done
}

# Point d'entrée
if [ "${BASH_SOURCE[0]}" = "${0}" ]; then
    check_requirements
    main "$@"
fi