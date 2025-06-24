#!/bin/bash
echo "🧹 Nettoyage des fichiers temporaires..."

# Supprimer les fichiers de build
cargo clean

# Supprimer les checkpoints
rm -f checkpoint_core_*.txt

# Supprimer les logs
rm -f solver.log

echo "✅ Nettoyage terminé."
