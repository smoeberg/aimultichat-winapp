# AI MultiChat WinApp (Desktop Client)

Dette er det selvstændige repository for den native **Windows-desktopklient** til **AI MultiChat**, bygget med **Tauri v2** og **Rust**.

## 🚀 Arkitektur & Tech Stack
- **Frontend / Launcher:** Letvægts statisk grænseflade, der forbinder direkte til jeres AIMultiChat-server.
- **Desktop Core:** **Rust** (Tauri v2, sikkerhed, system tray og native vindueshåndtering).
- **CI/CD:** GitHub Actions workflow til automatisk kompilering og pakning af `.msi` og `.exe` installatører.

---

## 🛠️ Lokal Udvikling

For at køre eller bygge WinApp'en lokalt på en Windows-maskine skal du have følgende installeret:
1. [Node.js](https://nodejs.org/) & [`pnpm`](https://pnpm.io/)
2. [Rust toolchain](https://rustup.rs/) (`rustup` / MSVC build tools)
3. Tauri CLI

### Start udviklingsmiljø:
```bash
# 1. Klon repositoriet
git clone https://github.com/smoeberg/aimultichat-winapp.git
cd aimultichat-winapp

# 2. Start i dev-mode
pnpm tauri dev
```

---

## ⚙️ Konfiguration (Backend URL)
Klienten forbinder som standard til `http://localhost:8000` (eller den URL, der er angivet i `winapp/frontend/index.html`). 

---

## 📦 Automatisk Byg & Release (CI/CD)
Pipelinen (`.github/workflows/aimultichat-winapp.yml`) bygger automatisk Windows-installatøren via GitHub Actions, når du opretter et release-tag:

```bash
git tag v1.0.0
git push origin v1.0.0
```

Når workflow-jobbet er fuldført (under **Actions**-fanen), kan de færdige `.msi`- og `.exe`-filer hentes direkte under **Releases** på GitHub.

---

## 🧩 Desktop Companion-funktioner (Lag 1)

- **System tray**: App'en indeholder et tray-ikon mer menu (**Åbn / Skjul / Afslut**). Venstre-klik på ikonet toggler chat-vinduet.
- **Global genvejstast:** `Ctrl+Shift+E` åbner/skjuler chat-vinduet fra enhver app.
- **CSP:** App'en bruger en fast content security policy (ingen `csp: null`).
