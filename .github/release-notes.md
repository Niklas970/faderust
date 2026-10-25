Testversion für Windows 10/11. Noch nicht auf echter Hardware erprobt, Fehler sind zu erwarten.

Neu in 0.4.4:

- Neues **Pult** im Stil von Voicemeeter: ein Streifen pro Gerät und pro Programm (z. B. Discord, Brave), jeweils mit einem Fader und Knöpfen A1, A2, A3 für die Ausgänge. Der Fader verschiebt alle eingeschalteten Ausgänge gemeinsam, ihr Abstand bleibt erhalten
- **Master-Fader pro Ausgang:** Links im Pult hat jeder Ausgang einen Gesamtregler und einen Stumm-Knopf (unter „Design“ auch rechts möglich)
- Neue Ansicht **Matrix**: alle Ein- und Ausgänge als Tabelle, jeder Pegel einzeln einstellbar. Umschalten unter „Design“
- „+ Programm“ sitzt jetzt direkt im Pult und in der Matrix, der Knopf „Programme“ oben entfällt, ebenso die Windows-Lautstärke-Leiste für Programme
- Neue Standardfarben: dunkle Darstellung und grüne Pegel (eigene Einstellungen bleiben erhalten)
- Bleibt auch mit drei oder mehr Ausgängen übersichtlich, kein seitliches Scrollen mehr

Neu in 0.4.3:

- Programm-Eingänge (z. B. Brave) werden nicht mehr grau, wenn das Programm eine Weile still ist. Sie bleiben verbunden, bis das Programm beendet wird, und spielen sofort weiter
- Ist ein Gerät belegt, sagt Faderust jetzt warum: Bei Bluetooth-Kopfhörern nutzt meist ein Programm wie Discord ihr Headset-Mikro (Freisprechmodus, dann ist der Stereo-Ausgang gesperrt). Sonst hält ein Programm das Gerät exklusiv fest, etwa eine DAW oder ein ASIO-Treiber
- Das Protokoll wiederholt einen solchen Fehler nicht mehr jede Sekunde

Neu in 0.4.2:

- Nur noch Bluetooth-Ausgänge geben sich bei Stille frei (für Multipoint). Kabel- und USB-Geräte bleiben offen und spielen nach einer Pause ohne Verzögerung weiter
- Weniger CPU-Last: Faderust fragt die Geräteliste nur noch ab, wenn ein Gerät fehlt (sonst alle 10 Sekunden), rechnet stumme oder ausgeschaltete Verbindungen nicht mehr um und schickt keine Pegel an das Fenster, solange es versteckt oder minimiert ist
- Die Programm-Pegel werden nur noch abgefragt, wenn die Programm-Leiste offen ist

Neu in 0.4.1:

- Ausgänge geben sich jetzt immer automatisch frei: nach 5 Sekunden Stille, ohne Einstellung. Die Auswahl „Frei nach …“ unter **Geräte** entfällt
- Kommt wieder Ton, öffnet Faderust den Ausgang sofort (Prüfung alle 50 ms statt jede Sekunde). Bei Bluetooth braucht die Verbindung selbst noch einen kurzen Moment
- Ein Klick auf die Versionsnummer oben links sucht sofort nach Updates

Neu in 0.4.0:

- **Schleifen-Schutz:** Spielt ein Ausgang auf das Kabel, von dem ein Eingang kommt (z. B. CABLE Input → CABLE Output), schaltet Faderust diese Verbindung stumm und zeigt „Schleife“ am Fader
- **Kein Doppelt-Hören:** „+ Eingang“ unter **Programme** stellt das Programm gleich auf ein freies Kabel, das du nicht hörst. Hörst du ein Programm trotzdem doppelt, zeigt der Dialog das und bietet die Lösung mit einem Klick
- **Programm-Mikro** für Discord, Teams & Co.: Unter **Geräte** legt „+ Programm-Mikro“ einen Ausgang auf einem freien virtuellen Kabel an (nur das Mikro ist an, alles andere stumm). Im Programm wählst du die andere Seite des Kabels als Mikrofon, auf Wunsch setzt Faderust sie gleich als Windows-Mikrofon für Anrufe
- **Aufgeräumt:** Die Programm-Lautstärken lassen sich rechts zu einer schmalen Leiste einklappen (Faderust merkt sich das). Programme, die schon Eingänge sind, stehen dort nicht mehr doppelt
- Fehlt ein virtuelles Kabel, öffnet „Virtuelles Kabel einrichten“ die Download-Seite von VB-Cable

Neu in 0.3.8:

- Ausgänge können sich bei Stille freigeben: Unter **Geräte** pro Ausgang „Frei nach 10 s“ bis „Frei nach 10 min“ wählen. Läuft so lange nichts auf den Ausgang, schließt Faderust das Gerät, und Bluetooth-Kopfhörer können per Multipoint zum Handy wechseln. Kommt wieder Ton, öffnet Faderust das Gerät von selbst (die erste Sekunde fehlt dabei).

Neu in 0.3.7:

- Absturz behoben, wenn ein Programm mit „+ Eingang“ aufgenommen wird
- Stürzt Faderust beim Start einer Programm-Aufnahme trotzdem ab, lässt es dieses Programm beim nächsten Start aus und meldet das, statt erneut abzustürzen
- Faderust schreibt ein Protokoll nach `%APPDATA%\io.github.niklas970.faderust\faderust.log`, das bei Fehlermeldungen hilft

Neu in 0.3.6:

- Programme als eigene Eingänge: Unter **Programme** nimmt „+ Eingang“ ein Programm direkt auf (ab Windows 10 Version 2004). Es bekommt in jeder Spalte einen Fader und eigene Effekte wie EQ und Gate, ohne ein Kabel pro Programm. Damit du es nicht doppelt hörst, stell das Programm auf ein Gerät, das du nicht direkt hörst (z. B. VB-Cable).

Neu in 0.3.5:

- Faderust sucht auch im laufenden Betrieb nach Updates (alle 30 Minuten und wenn das Fenster wieder geöffnet wird), nicht nur beim Start
- Stream-Deck-Plugin: Fader per Drehregler (Stream Deck+) oder Taste, Stummschalten, Presets laden, Windows-Standardgerät wechseln (`com.niklas970.faderust.streamDeckPlugin` im Release)

Neu in 0.3.4:

- Fader gehen bis +12 dB
- Reihenfolge von Ein- und Ausgängen unter **Geräte** mit Pfeilen änderbar (Presets wandern mit)

Neu in 0.3.3:

- Programme haben eine Pegelanzeige, darunter steht, ob ihr Ton über Faderust läuft oder direkt rausgeht
- Gate schneidet Wortenden nicht mehr ab (längere Haltezeit, weicheres Ausblenden)
- Fader-Kappe „Graphit & Kupferkante“ mit dB-Wert oben und unten, beleuchtete MUTE-Tasten, breitere LED-Ketten
- Neuer Knopf **Design**: Akzent- und Signalfarbe frei wählbar, hell oder dunkel

Neu in 0.3.2: Faderust sucht beim Start nach Updates. Gibt es eine neue Version, erscheint oben „Update auf …“. Ein Klick lädt sie, prüft die Signatur und installiert sie. Diese Version muss noch einmal von Hand installiert werden.

Neu in 0.3.1:

- Eingänge werden bei kurzen Aussetzern nicht mehr ständig neu verbunden
- Pult-Design im Stil von Voicemeeter: breite Fader mit dB-Wert auf der Kappe, LED-Pegel, Ausgänge als A1, A2 …
- Programm-Lautstärken als eigene Fader auf der Hauptseite

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
