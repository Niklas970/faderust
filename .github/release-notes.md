Testversion für Windows 10/11. Noch nicht auf echter Hardware erprobt, Fehler sind zu erwarten.

Neu in 0.3:

- Effekte pro Eingang: Low-Cut, Drei-Band-EQ und Gate (Klick auf den Eingang oben)
- Presets für Fader-Stellungen
- Dialog **Windows**: Standardgeräte für Wiedergabe, Aufnahme und Anrufe, Lautstärke pro Gerät
- Lautstärke und Stumm pro Programm im Dialog **Programme**
- Mit Windows starten (Geräte → Start), nur eine Instanz
- Neues Design: Logo, eigene Kopfleiste, breitere Fader mit Mausrad
- Setup statt MSI, auf Deutsch oder Englisch je nach Windows-Sprache. Eine installierte 0.1 oder 0.2 wird dabei entfernt.

## Installation

1. `Faderust_*_x64-setup.exe` herunterladen und starten. Der Installer spricht Deutsch oder Englisch, je nach Windows-Sprache, und braucht keine Admin-Rechte.
2. Eine früher installierte MSI-Version entfernt er dabei selbst.
3. Der Installer ist nicht signiert. Windows SmartScreen warnt deshalb: „Weitere Informationen“ → „Trotzdem ausführen“.
4. Faderust aus dem Startmenü starten. Fehlt WebView2, lädt der Installer es nach.

Stream Deck (optional): `com.niklas970.faderust.streamDeckPlugin` doppelklicken, die Aktionen stehen dann unter „Faderust“. Braucht Stream Deck 7.1 oder neuer und ein laufendes Faderust.

Die Konfiguration liegt unter `%APPDATA%\io.github.niklas970.faderust\config.toml`.

## Was drin ist

- Eine Spalte pro Ausgang, ein Fader pro Eingang, Stumm-Taste und Pegelanzeige
- Geräte-Dialog für Ein- und Ausgänge, Puffer pro Ausgang
- Geräte dürfen im Betrieb kommen und gehen (z. B. Bluetooth-Kopfhörer)
- Läuft im Tray weiter, wenn das Fenster geschlossen wird
- Effekte, Presets, Meldungen bei Geräteproblemen
- Windows-Standardgeräte und Lautstärken direkt aus Faderust
- Programme auf eigene Fader: [VB-Cable](https://vb-audio.com/Cable/) installieren (kostenlos, nicht enthalten), unter **Programme** das Kabel als Eingang anlegen und das Programm auf „CABLE Input“ stellen

## Rückmeldung

Probleme bitte als Issue melden, mit Windows-Version, beteiligten Geräten und was passiert ist.
