#!/usr/bin/env python3
"""
Analyseur de Performance pour Bitcoin Puzzle Solver
Analyse les logs, génère des statistiques et des graphiques
"""

import re
import json
import datetime
import matplotlib.pyplot as plt
import pandas as pd
from pathlib import Path
import argparse
import sys

class PuzzleSolverAnalyzer:
    def __init__(self):
        self.stats_data = []
        self.found_keys = []
        
    def parse_log_file(self, log_file):
        """Parse les logs du solveur"""
        print(f"📊 Analyse du fichier: {log_file}")
        
        try:
            with open(log_file, 'r', encoding='utf-8') as f:
                content = f.read()
        except FileNotFoundError:
            print(f"❌ Fichier non trouvé: {log_file}")
            return False
            
        # Regex pour extraire les statistiques
        stats_pattern = r'\[Stats\] Total: (\d+) \| Vitesse: ([\d.]+) clés/s \| Instantané: ([\d.]+) clés/s \| Temps: (\d+):(\d+):(\d+) \| Cœurs: (\d+)'
        found_pattern = r'💰 ADRESSE TROUVÉE ! 💰.*?🔍 Adresse: ([^\n]+).*?🔑 Clé Privée \(WIF\): ([^\n]+).*?🔢 Nombre Décimal: ([^\n]+)'
        
        # Extraire les statistiques
        stats_matches = re.findall(stats_pattern, content)
        for match in stats_matches:
            total, avg_speed, instant_speed, hours, minutes, seconds, cores = match
            total_seconds = int(hours) * 3600 + int(minutes) * 60 + int(seconds)
            
            self.stats_data.append({
                'timestamp': datetime.datetime.now() - datetime.timedelta(seconds=len(self.stats_data)*10),
                'total_keys': int(total),
                'avg_speed': float(avg_speed),
                'instant_speed': float(instant_speed),
                'elapsed_seconds': total_seconds,
                'cores': int(cores)
            })
        
        # Extraire les clés trouvées
        found_matches = re.findall(found_pattern, content, re.DOTALL)
        for match in found_matches:
            address, wif, decimal = match
            self.found_keys.append({
                'address': address.strip(),
                'wif': wif.strip(),
                'decimal': decimal.strip(),
                'timestamp': datetime.datetime.now()
            })
        
        print(f"✅ Trouvé {len(self.stats_data)} entrées de statistiques")
        print(f"✅ Trouvé {len(self.found_keys)} clés résolues")
        return True
    
    def generate_statistics(self):
        """Génère des statistiques détaillées"""
        if not self.stats_data:
            print("❌ Aucune donnée statistique disponible")
            return
        
        df = pd.DataFrame(self.stats_data)
        
        print("\n📈 STATISTIQUES GÉNÉRALES")
        print("=" * 50)
        print(f"Nombre total de clés testées: {df['total_keys'].max():,}")
        print(f"Vitesse moyenne: {df['avg_speed'].mean():.2f} clés/s")
        print(f"Vitesse maximale: {df['avg_speed'].max():.2f} clés/s")
        print(f"Vitesse minimale: {df['avg_speed'].min():.2f} clés/s")
        print(f"Temps total d'exécution: {df['elapsed_seconds'].max()} secondes")
        print(f"Nombre de cœurs utilisés: {df['cores'].iloc[-1] if len(df) > 0 else 'N/A'}")
        
        # Estimation du temps pour résoudre différents puzzles
        avg_speed = df['avg_speed'].mean()
        print(f"\n⏱️  ESTIMATIONS TEMPORELLES (à {avg_speed:.0f} clés/s)")
        print("=" * 50)
        
        puzzles = [
            (64, 2**63, 2**64-1),
            (66, 2**65, 2**66-1),
            (68, 2**67, 2**68-1),
            (70, 2**69, 2**70-1),
        ]
        
        for puzzle_num, start, end in puzzles:
            total_keys = end - start + 1
            time_seconds = total_keys / (avg_speed * 2)  # Estimation probabiliste
            
            if time_seconds < 3600:
                time_str = f"{time_seconds/60:.1f} minutes"
            elif time_seconds < 86400:
                time_str = f"{time_seconds/3600:.1f} heures"
            elif time_seconds < 31536000:
                time_str = f"{time_seconds/86400:.1f} jours"
            else:
                time_str = f"{time_seconds/31536000:.1f} années"
            
            print(f"Puzzle {puzzle_num}: ~{time_str} (espace: 2^{puzzle_num-1} clés)")
        
        return df
    
    def create_performance_graph(self, df, output_file="performance.png"):
        """Crée un graphique de performance"""
        if df.empty:
            print("❌ Pas de données pour le graphique")
            return
        
        plt.figure(figsize=(12, 8))
        
        # Graphique de vitesse dans le temps
        plt.subplot(2, 2, 1)
        plt.plot(df.index, df['avg_speed'], label='Vitesse moyenne', color='blue')
        plt.plot(df.index, df['instant_speed'], label='Vitesse instantanée', color='red', alpha=0.7)
        plt.title('Vitesse de Recherche')
        plt.xlabel('Temps (échantillons)')
        plt.ylabel('Clés/seconde')
        plt.legend()
        plt.grid(True)
        
        # Graphique cumulatif des clés testées
        plt.subplot(2, 2, 2)
        plt.plot(df.index, df['total_keys'], color='green')
        plt.title('Clés Testées (Cumulatif)')
        plt.xlabel('Temps (échantillons)')
        plt.ylabel('Nombre de clés')
        plt.grid(True)
        
        # Histogramme des vitesses
        plt.subplot(2, 2, 3)
        plt.hist(df['avg_speed'], bins=20, alpha=0.7, color='purple')
        plt.title('Distribution des Vitesses')
        plt.xlabel('Clés/seconde')
        plt.ylabel('Fréquence')
        plt.grid(True)
        
        # Efficacité par cœur
        plt.subplot(2, 2, 4)
        efficiency = df['avg_speed'] / df['cores']
        plt.plot(df.index, efficiency, color='orange')
        plt.title('Efficacité par Cœur')
        plt.xlabel('Temps (échantillons)')
        plt.ylabel('Clés/seconde/cœur')
        plt.grid(True)
        
        plt.tight_layout()
        plt.savefig(output_file, dpi=300, bbox_inches='tight')
        print(f"📊 Graphique sauvegardé: {output_file}")
        
    def export_results(self, output_file="results.json"):
        """Exporte les résultats en JSON"""
        results = {
            'statistics': self.stats_data,
            'found_keys': self.found_keys,
            'analysis_date': datetime.datetime.now().isoformat(),
        }
        
        with open(output_file, 'w', encoding='utf-8') as f:
            json.dump(results, f, indent=2, default=str)
        
        print(f"💾 Résultats exportés: {output_file}")
    
    def generate_report(self, output_file="report.html"):
        """Génère un rapport HTML complet"""
        html_content = f"""
        <!DOCTYPE html>
        <html>
        <head>
            <title>Bitcoin Puzzle Solver - Rapport d'Analyse</title>
            <style>
                body {{ font-family: Arial, sans-serif; margin: 20px; }}
                .header {{ background: linear-gradient(45deg, #f39c12, #e67e22); color: white; padding: 20px; border-radius: 10px; }}
                .section {{ margin: 20px 0; padding: 15px; border-left: 4px solid #3498db; background: #f8f9fa; }}
                .stats {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 15px; }}
                .stat-card {{ background: white; padding: 15px; border-radius: 8px; box-shadow: 0 2px 4px rgba(0,0,0,0.1); }}
                .found-key {{ background: #d4edda; padding: 10px; margin: 10px 0; border-radius: 5px; border-left: 4px solid #28a745; }}
            </style>
        </head>
        <body>
            <div class="header">
                <h1>🚀 Bitcoin Puzzle Solver - Rapport d'Analyse</h1>
                <p>Généré le: {datetime.datetime.now().strftime('%Y-%m-%d %H:%M:%S')}</p>
            </div>
            
            <div class="section">
                <h2>📊 Statistiques Générales</h2>
                <div class="stats">
        """
        
        if self.stats_data:
            df = pd.DataFrame(self.stats_data)
            html_content += f"""
                    <div class="stat-card">
                        <h3>Clés Testées</h3>
                        <p><strong>{df['total_keys'].max():,}</strong></p>
                    </div>
                    <div class="stat-card">
                        <h3>Vitesse Moyenne</h3>
                        <p><strong>{df['avg_speed'].mean():.2f}</strong> clés/s</p>
                    </div>
                    <div class="stat-card">
                        <h3>Vitesse Maximale</h3>
                        <p><strong>{df['avg_speed'].max():.2f}</strong> clés/s</p>
                    </div>
                    <div class="stat-card">
                        <h3>Temps d'Exécution</h3>
                        <p><strong>{df['elapsed_seconds'].max()}</strong> secondes</p>
                    </div>
            """
        
        html_content += """
                </div>
            </div>
        """
        
        if self.found_keys:
            html_content += """
            <div class="section">
                <h2>🎉 Clés Trouvées</h2>
            """
            for key in self.found_keys:
                html_content += f"""
                <div class="found-key">
                    <h4>Adresse: {key['address']}</h4>
                    <p><strong>WIF:</strong> {key['wif']}</p>
                    <p><strong>Décimal:</strong> {key['decimal']}</p>
                </div>
                """
            html_content += "</div>"
        
        html_content += """
        </body>
        </html>
        """
        
        with open(output_file, 'w', encoding='utf-8') as f:
            f.write(html_content)
        
        print(f"📄 Rapport HTML généré: {output_file}")

def main():
    parser = argparse.ArgumentParser(description="Analyseur de Performance Bitcoin Puzzle Solver")
    parser.add_argument('--log', default='solver.log', help='Fichier de log à analyser')
    parser.add_argument('--output', default='analysis', help='Préfixe des fichiers de sortie')
    parser.add_argument('--graph', action='store_true', help='Générer des graphiques')
    parser.add_argument('--report', action='store_true', help='Générer un rapport HTML')
    
    args = parser.parse_args()
    
    analyzer = PuzzleSolverAnalyzer()
    
    print("🔍 Bitcoin Puzzle Solver - Analyseur de Performance")
    print("=" * 60)
    
    # Essayer de parser différents types de fichiers
    log_files = [args.log, 'solver.log', 'output.log', 'bitcoin_solver.log']
    parsed = False
    
    for log_file in log_files:
        if Path(log_file).exists():
            if analyzer.parse_log_file(log_file):
                parsed = True
                break
    
    if not parsed:
        print("❌ Aucun fichier de log trouvé. Fichiers recherchés:")
        for f in log_files:
            print(f"   - {f}")
        sys.exit(1)
    
    # Générer les statistiques
    df = analyzer.generate_statistics()
    
    # Exporter les résultats
    analyzer.export_results(f"{args.output}.json")
    
    # Générer les graphiques si demandé
    if args.graph and df is not None:
        try:
            analyzer.create_performance_graph(df, f"{args.output}_performance.png")
        except ImportError:
            print("⚠️  matplotlib non installé. Graphiques non générés.")
            print("   Installez avec: pip install matplotlib pandas")
    
    # Générer le rapport HTML si demandé
    if args.report:
        analyzer.generate_report(f"{args.output}_report.html")
    
    print(f"\n✅ Analyse terminée!")
    print(f"📁 Fichiers générés:")
    print(f"   - {args.output}.json")
    if args.graph:
        print(f"   - {args.output}_performance.png")
    if args.report:
        print(f"   - {args.output}_report.html")

if __name__ == "__main__":
    main()