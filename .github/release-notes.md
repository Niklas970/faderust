Testversion für Windows 10/11. Noch nicht auf echter Hardware erprobt, Fehler sind zu erwarten.

Neu in 0.2: Dialog **Programme**. Dort bekommt jedes Programm (Spotify, Discord, Browser …) sein eigenes Ausgabegerät, etwa ein virtuelles Kabel, und damit einen eigenen Fader. Eine installierte 0.1 wird beim Installieren ersetzt.

## Installation

1. `Faderust_*.msi` herunterladen und starten.
2. Die MSI ist nicht signiert. Windows SmartScreen warnt deshalb: „Weitere Informationen“ → „Trotzdem ausführen“.
3. Faderust aus dem Startmenü starten. Fehlt WebView2, lädt der Installer es nach.

Die Konfiguration liegt unter `%APPDATA%\io.github.niklas970.faderust\config.toml`.

## Was drin ist

- Eine Spalte pro Ausgang, ein Fader pro Eingang, Stumm-Taste und Pegelanzeige
- Geräte-Dialog für Ein- und Ausgänge, Puffer pro Ausgang
- Geräte dürfen im Betrieb kommen und gehen (z. B. Bluetooth-Kopfhörer)
- Läuft im Tray weiter, wenn das Fenster geschlossen wird
- Programme auf eigene Fader: [VB-Cable](https://vb-audio.com/Cable/) installieren (kostenlos, nicht enthalten), unter **Programme** das Kabel als Eingang anlegen und das Programm auf „CABLE Input“ stellen

## Rückmeldung

Probleme bitte als Issue melden, mit Windows-Version, beteiligten Geräten und was passiert ist.
