<?php
header('Content-Type: application/json');
header('Access-Control-Allow-Origin: *');
header('Access-Control-Allow-Methods: GET, POST');
header('Access-Control-Allow-Headers: Content-Type');

// Configuration
$SCRIPT_NAME = 'bitcoin_solver'; // Nom de votre exécutable
$LOG_FILE = 'bitcoin_solver.log'; // Fichier de log de sortie
$FOUND_FILE = 'found.txt'; // Fichier des clés trouvées
$CONFIG_FILE = 'config.txt'; // Fichier de configuration
$CHECKPOINT_DIR = './'; // Répertoire des fichiers checkpoint

// Classe principale pour le monitoring
class BitcoinSolverMonitor {
    private $scriptName;
    private $logFile;
    private $foundFile;
    private $configFile;
    private $checkpointDir;
    
    public function __construct($scriptName, $logFile, $foundFile, $configFile, $checkpointDir) {
        $this->scriptName = $scriptName;
        $this->logFile = $logFile;
        $this->foundFile = $foundFile;
        $this->configFile = $configFile;
        $this->checkpointDir = $checkpointDir;
    }
    
    public function getStatus() {
        $status = [];
        
        // Vérifier si le processus est en cours d'exécution
        $running = $this->isProcessRunning();
        $status['running'] = $running;
        
        if ($running) {
            $status['pid'] = $this->getProcessPID();
            $status['cpu_usage'] = $this->getCPUUsage($status['pid']);
            $status['memory_usage'] = $this->getMemoryUsage($status['pid']);
        }
        
        // Analyser les logs pour extraire les statistiques
        $logStats = $this->parseLogStats();
        $status = array_merge($status, $logStats);
        
        // Lire la configuration
        $config = $this->readConfig();
        $status['cores'] = $config['cores'] ?? 'Auto';
        $status['search_mode'] = $config['mode'] ?? 'N/A';
        $status['range'] = ($config['start'] ?? 'N/A') . ' - ' . ($config['end'] ?? 'N/A');
        
        // Compter les clés trouvées
        $status['found_count'] = $this->countFoundKeys();
        
        // Calculer le temps de fonctionnement
        $status['uptime'] = $this->getUptime();
        
        return $status;
    }
    
    public function getLogs($from = 0) {
        if (!file_exists($this->logFile)) {
            return "Log file not found. Le script n'a peut-être pas encore généré de logs.\n";
        }
        
        $content = file_get_contents($this->logFile);
        if ($from > 0) {
            $content = substr($content, $from);
        }
        
        // Limiter à 50KB pour éviter les problèmes de mémoire
        if (strlen($content) > 50000) {
            $content = substr($content, -50000);
        }
        
        return $content;
    }
    
    public function stopScript() {
        $pid = $this->getProcessPID();
        if ($pid) {
            // Essayer d'arrêter proprement avec SIGTERM
            exec("kill -TERM $pid 2>&1", $output, $return_code);
            sleep(2);
            
            // Vérifier si le processus est toujours en cours
            if ($this->isProcessRunning()) {
                // Forcer l'arrêt avec SIGKILL
                exec("kill -KILL $pid 2>&1", $output2, $return_code2);
                return ['success' => true, 'message' => 'Processus forcé à s\'arrêter'];
            }
            
            return ['success' => true, 'message' => 'Processus arrêté proprement'];
        }
        
        return ['success' => false, 'error' => 'Processus non trouvé'];
    }
    
    private function isProcessRunning() {
        exec("pgrep -f {$this->scriptName}", $output);
        return !empty($output);
    }
    
    private function getProcessPID() {
        exec("pgrep -f {$this->scriptName}", $output);
        return !empty($output) ? $output[0] : null;
    }
    
    private function getCPUUsage($pid) {
        if (!$pid) return 'N/A';
        
        exec("ps -p $pid -o %cpu --no-headers 2>/dev/null", $output);
        return !empty($output) ? trim($output[0]) . '%' : 'N/A';
    }
    
    private function getMemoryUsage($pid) {
        if (!$pid) return 'N/A';
        
        exec("ps -p $pid -o %mem,rss --no-headers 2>/dev/null", $output);
        if (!empty($output)) {
            $parts = preg_split('/\s+/', trim($output[0]));
            $memPercent = $parts[0] ?? '0';
            $memKB = $parts[1] ?? '0';
            $memMB = round($memKB / 1024, 1);
            return "{$memPercent}% ({$memMB}MB)";
        }
        return 'N/A';
    }
    
    private function parseLogStats() {
        $stats = [
            'keys_checked' => 0,
            'speed' => '0 clés/s',
            'last_stats_line' => ''
        ];
        
        if (!file_exists($this->logFile)) {
            return $stats;
        }
        
        // Lire les dernières lignes du fichier de log
        $lines = $this->tail($this->logFile, 50);
        
        foreach (array_reverse($lines) as $line) {
            // Chercher la ligne de statistiques la plus récente
            if (preg_match('/Total:\s*(\d+).*?Vitesse:\s*([\d,]+\.?\d*)\s*clés\/s/', $line, $matches)) {
                $stats['keys_checked'] = intval(str_replace(',', '', $matches[1]));
                $stats['speed'] = $matches[2] . ' clés/s';
                $stats['last_stats_line'] = $line;
                break;
            }
        }
        
        return $stats;
    }
    
    private function readConfig() {
        $config = [];
        
        if (!file_exists($this->configFile)) {
            return $config;
        }
        
        $lines = file($this->configFile, FILE_IGNORE_NEW_LINES | FILE_SKIP_EMPTY_LINES);
        
        foreach ($lines as $line) {
            $line = trim($line);
            if (empty($line) || $line[0] === '#') {
                continue;
            }
            
            if (strpos($line, '=') !== false) {
                list($key, $value) = explode('=', $line, 2);
                $config[trim($key)] = trim($value);
            }
        }
        
        return $config;
    }
    
    private function countFoundKeys() {
        if (!file_exists($this->foundFile)) {
            return 0;
        }
        
        $content = file_get_contents($this->foundFile);
        return substr_count($content, 'Trouvé!');
    }
    
    private function getUptime() {
        $pid = $this->getProcessPID();
        if (!$pid) return '00:00:00';
        
        exec("ps -p $pid -o etime --no-headers 2>/dev/null", $output);
        return !empty($output) ? trim($output[0]) : '00:00:00';
    }
    
    private function tail($filename, $lines = 100) {
        if (!file_exists($filename)) {
            return [];
        }
        
        $handle = fopen($filename, "r");
        if (!$handle) {
            return [];
        }
        
        $linecounter = $lines;
        $pos = -2;
        $beginning = false;
        $text = [];
        
        while ($linecounter > 0) {
            $t = " ";
            while ($t != "\n") {
                if (fseek($handle, $pos, SEEK_END) == -1) {
                    $beginning = true;
                    break;
                }
                $t = fgetc($handle);
                $pos--;
            }
            
            $linecounter--;
            if ($beginning) {
                rewind($handle);
            }
            
            $text[$lines - $linecounter - 1] = fgets($handle);
            if ($beginning) break;
        }
        
        fclose($handle);
        return array_reverse($text);
    }
}

// Traitement des requêtes
$action = $_GET['action'] ?? '';
$monitor = new BitcoinSolverMonitor($SCRIPT_NAME, $LOG_FILE, $FOUND_FILE, $CONFIG_FILE, $CHECKPOINT_DIR);

switch ($action) {
    case 'get_status':
        echo json_encode($monitor->getStatus());
        break;
        
    case 'get_logs':
        header('Content-Type: text/plain');
        $from = isset($_GET['from']) ? intval($_GET['from']) : 0;
        echo $monitor->getLogs($from);
        break;
        
    case 'stop_script':
        if ($_SERVER['REQUEST_METHOD'] === 'POST') {
            echo json_encode($monitor->stopScript());
        } else {
            echo json_encode(['success' => false, 'error' => 'Method not allowed']);
        }
        break;
        
    case 'download_logs':
        if (file_exists($LOG_FILE)) {
            header('Content-Type: application/octet-stream');
            header('Content-Disposition: attachment; filename="bitcoin_solver_' . date('Y-m-d_H-i-s') . '.log"');
            readfile($LOG_FILE);
        } else {
            header('Content-Type: text/plain');
            echo "Log file not found";
        }
        break;
        
    case 'get_found_keys':
        if (file_exists($FOUND_FILE)) {
            header('Content-Type: text/plain');
            readfile($FOUND_FILE);
        } else {
            header('Content-Type: text/plain');
            echo "No keys found yet";
        }
        break;
        
    default:
        echo json_encode([
            'error' => 'Action not specified or invalid',
            'available_actions' => [
                'get_status',
                'get_logs',
                'stop_script',
                'download_logs',
                'get_found_keys'
            ]
        ]);
}
?>