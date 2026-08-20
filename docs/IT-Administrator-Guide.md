# IT-Administrator Guide: AI MultiChat WinApp Deployment

Dette dokument beskriver systemkrav, installation og distributionsparametre for **AI MultiChat Windows Desktop Client** (`aimultichat-winapp`).

---

## 1. Systemkrav
- **Operativsystem:** Windows 10 eller Windows 11 (64-bit).
- **Runtime:** Microsoft Edge WebView2 Runtime (forinstalleret på Windows 11 og de fleste opdaterede Windows 10-maskiner). Hvis WebView2 mangler, henvises der automatisk til Microsofts bootstrapper.
- **Hardware:** Min. 2 GB RAM, 100 MB ledig diskplads.

---

## 2. Installationsfiler (Artefakter)
Ved hvert GitHub Release (`v*`) genereres to formater:
1. **`.msi` (Microsoft Installer):** Ideel til Active Directory / Intune / MSI-baseret udrulning.
2. **`.exe` (NSIS Installer):** Standard enkeltbruger-installatør.

---

## 3. Silent Installation (Udrulning via Intune / SCCM)

### For `.msi`-pakken:
For at installere programmet lydløst (silent) for alle brugere eller i baggrunden:

```cmd
msiexec /i AIMultiChat_1.0.0_x64_en-US.msi /quiet /norestart
```

For at afinstallere lydløst:
```cmd
msiexec /x AIMultiChat_1.0.0_x64_en-US.msi /quiet /norestart
```

### For `.exe`-pakken (NSIS):
NSIS-installatøren understøtter standard silent-flag:
```cmd
AIMultiChat_1.0.0_x64-setup.exe /S
```
Afinstallation:
```cmd
"%PROGRAMFILES%\AIMultiChat\uninstall.exe" /S
```

---

## 4. Netværk & Firewall Krav
App'en er en native desktop-klient, der kommunikerer med AIMultiChat-backendserveren.
- **Udgående porte:** HTTPS (TCP 443) til serverens domæne.
- **Lokale genveje:** `Ctrl+Shift+E` (Global hotkey til at fremkalde chat-vinduet).
