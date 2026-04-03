<div align="center">

<svg xmlns="http://www.w3.org/2000/svg" width="120" height="120" viewBox="0 0 120 120">
  <defs>
    <linearGradient id="g" x1="0%" y1="0%" x2="100%" y2="100%">
      <stop offset="0%" stop-color="#00d4ff"/>
      <stop offset="100%" stop-color="#7c3aed"/>
    </linearGradient>
  </defs>
  <circle cx="46" cy="46" r="22" fill="none" stroke="url(#g)" stroke-width="4"/>
  <line x1="63" y1="63" x2="95" y2="95" stroke="url(#g)" stroke-width="7" stroke-linecap="round"/>
</svg>

# SHERLOCK-RS

**Hunt down social media accounts by username — Rust Edition**

[![Rust](https://img.shields.io/badge/Rust-1.94+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/License-MIT-blue?style=flat-square)](LICENSE)
[![Sites](https://img.shields.io/badge/Sites-478+-brightgreen?style=flat-square)](https://github.com/sherlock-project/sherlock)
[![Platform](https://img.shields.io/badge/Platform-Windows-0078D4?style=flat-square&logo=windows)](https://github.com/Oli97430/sherlock-rs/releases)

*A complete Rust rewrite of [Sherlock](https://github.com/sherlock-project/sherlock) with a modern dark web UI — single `.exe`, no installation required.*

</div>

---

## Aperçu

**Sherlock-RS** scanne **478+ plateformes sociales** simultanément pour détecter si un nom d'utilisateur existe. Il suffit de lancer l'exe : un serveur local démarre et le navigateur s'ouvre automatiquement avec une interface moderne.

![Interface sombre avec barre de progression et résultats en temps réel](https://raw.githubusercontent.com/Oli97430/sherlock-rs/main/docs/screenshot.png)

---

## Fonctionnalités

| Fonctionnalité | Détail |
|---|---|
| 🔍 **478+ sites** | Base de données Sherlock officielle, mise à jour en un clic |
| ⚡ **Parallélisme** | 20 requêtes simultanées via Tokio async |
| 🎨 **UI moderne** | Interface web dark theme avec résultats en temps réel (SSE) |
| 🛡️ **Détection WAF** | Cloudflare, PerimeterX, AWS CloudFront |
| 🧅 **Proxy / Tor** | Support SOCKS5 (`socks5://127.0.0.1:9050`) |
| 📥 **Export** | CSV et TXT |
| 🔎 **Filtrage** | Tri par nom / statut / temps de réponse, filtre textuel |
| 📦 **Zéro install** | Un seul `.exe` de 5 MB, aucune dépendance |

---

## Installation

### Méthode rapide — Télécharger l'exe

Télécharge la dernière version depuis [**Releases**](https://github.com/Oli97430/sherlock-rs/releases), double-clique sur `sherlock-rs.exe`.

### Compiler depuis les sources

**Prérequis :** [Rust](https://rustup.rs/) + [Visual Studio Build Tools](https://visualstudio.microsoft.com/fr/downloads/) (Windows)

```bash
git clone https://github.com/Oli97430/sherlock-rs.git
cd sherlock-rs
cargo build --release
```

L'exe se trouve dans `target/release/sherlock-rs.exe`.

---

## Utilisation

```bash
sherlock-rs.exe
```

1. L'exe démarre un serveur local sur un port aléatoire
2. Le navigateur s'ouvre automatiquement sur l'interface
3. Entre un nom d'utilisateur et clique **Hunt**
4. Les résultats arrivent en temps réel

### Options disponibles dans l'UI

| Option | Description |
|---|---|
| **Timeout** | Délai max par requête (défaut : 30s) |
| **NSFW** | Inclure les plateformes adultes |
| **Proxy** | Ex: `socks5://127.0.0.1:9050` pour Tor |
| **Update DB** | Télécharge la dernière base de données depuis GitHub |

### Raccourcis clavier

| Touche | Action |
|---|---|
| `Enter` | Lancer la recherche |
| `Escape` | Stopper la recherche |

---

## Architecture

```
sherlock-rs/
├── Cargo.toml              # Dépendances Rust
├── src/
│   ├── main.rs             # Point d'entrée + banner console
│   ├── server.rs           # Serveur Axum (API REST + SSE streaming)
│   ├── checker.rs          # Moteur de scan async (20 workers)
│   ├── sites.rs            # Chargement data.json (cache + GitHub)
│   ├── result.rs           # Types : QueryStatus, QueryResult
│   └── export.rs           # Export CSV / TXT
└── frontend/
    └── index.html          # UI complète embarquée dans le binaire
```

### Stack technique

| Composant | Crate |
|---|---|
| Runtime async | `tokio` |
| Serveur web | `axum 0.7` |
| HTTP client | `reqwest 0.12` |
| Sérialisation | `serde` + `serde_json` |
| Regex | `regex` |
| Export CSV | `csv` |
| Open navigateur | `open` |
| Erreurs | `anyhow` |

---

## Méthodes de détection

Sherlock-RS implémente les 3 méthodes du projet original :

| Type | Logique |
|---|---|
| `status_code` | Réponse 404 (ou code configuré) = absent ; 200-299 = présent |
| `message` | Texte d'erreur trouvé dans le body = absent |
| `response_url` | Redirection désactivée ; 200-299 = présent |

La détection WAF (Cloudflare, PerimeterX…) est appliquée avant toute autre logique pour éviter les faux positifs.

---

## Statuts des résultats

| Statut | Signification |
|---|---|
| ✅ **Found** | Compte détecté sur la plateforme |
| ❌ **Not Found** | Aucun compte à ce nom |
| ⚠️ **Blocked** | Bloqué par un WAF (essaie avec un proxy) |
| 🔴 **Error** | Erreur réseau ou timeout |
| ⬜ **Invalid** | Format du nom invalide pour ce site |

---

## Crédits

- **Auteur** : Olivier Hoarau — [tarraw974@gmail.com](mailto:tarraw974@gmail.com)
- **Inspiré de** : [Sherlock Project](https://github.com/sherlock-project/sherlock) (MIT License)
- **Base de données** : `data.json` maintenu par la communauté Sherlock

---

## Licence

MIT — voir [LICENSE](LICENSE)

---

<div align="center">
  <sub>Made with ❤️ and Rust 🦀</sub>
</div>
