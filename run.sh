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
