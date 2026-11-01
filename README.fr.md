<p align="center"><a href="README.md">English</a> · <a href="README.de.md">Deutsch</a> · <b>Français</b> · <a href="README.es.md">Español</a> · <a href="README.it.md">Italiano</a> · <a href="README.nl.md">Nederlands</a> · <a href="README.pl.md">Polski</a></p>

<p align="center"><img src="assets/logo.svg" width="112" alt="Faderust logo"></p>

<h1 align="center">Faderust</h1>

<p align="center">
  Mixeur audio pour Windows, un remplaçant léger et ouvert de Voicemeeter.<br>
  Chaque sortie a son propre mix, les appareils peuvent aller et venir en cours d’utilisation, et Faderust apporte ses propres câbles virtuels.
</p>

<p align="center">
  <a href="https://github.com/Niklas970/faderust/releases/latest"><img src="https://img.shields.io/github/v/release/Niklas970/faderust?label=Download" alt="Dernière version"></a>
  <a href="https://github.com/Niklas970/faderust/actions/workflows/ci.yml"><img src="https://github.com/Niklas970/faderust/actions/workflows/ci.yml/badge.svg" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/Niklas970/faderust" alt="Licence MIT"></a>
</p>

![Table de mixage Faderust avec appareils, programmes et trois sorties](assets/screenshot.png)

<p align="center">
  <a href="assets/faderust-reel.mp4"><img src="assets/faderust-reel.gif" width="270" alt="Faderust en 40 secondes"></a><br>
  <sub>Aperçu, <a href="assets/faderust-reel.mp4">vidéo en pleine qualité</a> (à télécharger)</sub>
</p>

> **Alpha.** Faderust fonctionne, mais a encore à peine été testé sur du vrai matériel. Merci de signaler les problèmes via une [issue](https://github.com/Niklas970/faderust/issues).

## Ce que fait Faderust

- **Une table comme Voicemeeter.** Chaque appareil et chaque programme a une tranche avec fader, MUTE et les boutons A1, A2, A3 pour les sorties. Chaque sortie a son propre fader master.
- **Matrice pour le travail de précision.** Sous forme de tableau : chaque entrée réglée séparément sur chaque sortie, par exemple le micro plus faible dans le casque que dans le stream.
- **Capturer les programmes directement.** Discord, Spotify ou le navigateur ont leur propre tranche avec effets, sans câble virtuel par programme. Faderust masque les programmes silencieux et les rappelle individuellement sur demande (« N silencieux, afficher »).
- **Ses propres câbles virtuels, sans pilote supplémentaire.** L’application installe elle-même le « Faderust Kabel » (jusqu’à quatre), par exemple un pour le micro et un pour les programmes. Détails sous [Faderust Kabel](#faderust-kabel).
- **Rangé.** Les sorties se masquent avec le × et reviennent d’elles-mêmes dès que du son y est de nouveau lu.
- **Les appareils vont et viennent.** Éteindre puis rallumer un casque Bluetooth, débrancher un appareil USB : Faderust se reconnecte sans interrompre les autres appareils.
- **Compatible multipoint.** Faderust libère les sorties Bluetooth pendant le silence (après 30 s, réglable par sortie) pour que le casque puisse passer sur le téléphone. Quand le son revient, elles se rouvrent d’elles-mêmes.
- **Effets par entrée :** réduction de bruit ([RNNoise](https://jmvalin.ca/demo/rnnoise/), contre le clavier et le ventilateur), filtre passe-haut, égaliseur 3 bandes et gate. La réduction de bruit fonctionne sur les appareils à 48 kHz et retarde le son de 10 ms.
- **Allemand et anglais.** La langue suit Windows et se change sous « Design ».
- **Les réglages s’enregistrent tout seuls.** La boîte de dialogue des appareils applique les modifications peu après la dernière saisie.
- **Autotest.** Sous « Appareils », Faderust vérifie les appareils, les tampons, le bouclage, la télécommande et le plugin Stream Deck et indique ce qui est en ordre ou non. Le rapport peut être copié pour une issue.
- **Le son Windows au même endroit :** appareils par défaut pour la lecture, l’enregistrement et les appels, ainsi que le volume par appareil.
- **Presets, démarrage automatique dans la zone de notification, mises à jour dans l’application** (avec vérification de signature) et un **plugin Stream Deck**.
- **Compact et au premier plan.** Deux boutons dans la barre de titre : une vue étroite sans libellés et « Toujours au premier plan », par exemple à côté d’un jeu. Les deux sont mémorisés.
- **Vos propres couleurs :** couleur d’accent et de signal au choix, sombre (noir OLED) ou clair.

## Vues

<table>
  <tr>
    <td width="68%"><img src="assets/matrix.png" alt="Matrice : chaque entrée réglée séparément sur chaque sortie"></td>
    <td><img src="assets/design.png" alt="Boîte de dialogue Design avec vue, couleurs et apparence"></td>
  </tr>
  <tr>
    <td><b>Matrice :</b> chaque entrée sur chaque sortie séparément, par exemple le micro plus faible dans le casque que dans le stream.</td>
    <td><b>Design :</b> table ou matrice, côté des sorties, couleurs, sombre ou clair.</td>
  </tr>
</table>

## Installation

1. Téléchargez `Faderust_*_x64-setup.exe` depuis les [Releases](https://github.com/Niklas970/faderust/releases/latest) et lancez-le. Le programme d’installation ne demande pas de droits administrateur et parle allemand ou anglais.
2. Le programme d’installation n’est pas signé, Windows SmartScreen affiche donc un avertissement : « Informations complémentaires » → « Exécuter quand même ».
3. Ouvrez Faderust depuis le menu Démarrer. L’application signale les nouvelles versions en haut de la barre de titre ; un clic sur le numéro de version lance aussitôt une recherche.

Prérequis : Windows 10 (à partir de la version 2004) ou Windows 11.

## Premiers pas

1. Ouvrez **Appareils** et créez des entrées (micro, ligne, Faderust Kabel) et des sorties (casque, enceintes, stream). Le Bluetooth demande environ 80 ms de tampon, l’USB le plus souvent 30 à 60 ms.
2. Dans la **table**, choisissez avec A1–A3 où une entrée est lue. Le fader règle ensemble toutes les sorties activées.
3. Utiliser les faders : glisser, molette de la souris (plus fin avec Maj), double-clic pour 0 dB, ou cliquer sur la valeur en dB en dessous et saisir un nombre.
4. **Effets** sous une tranche ouvre le filtre passe-haut, l’égaliseur et le gate.
5. Sous **Design**, basculez entre table et matrice et choisissez le côté des sorties et les couleurs.

### Programmes

**+ Programme** dans la table affiche tous les programmes qui jouent du son en ce moment. **+ Entrée** en capture un directement. Pour que vous ne l’entendiez pas deux fois, Faderust place d’abord le programme sur un Faderust Kabel libre que vous n’écoutez pas (s’il est installé), puis le capture. Un programme actuellement silencieux est masqué par Faderust ; **N silencieux, afficher** le rappelle.

### Micro pour Discord, Teams & Cie

Sous **Appareils**, **+ Micro de programme** crée une sortie sur un Faderust Kabel libre qui ne porte que le micro avec ses effets. Dans le programme, choisissez le côté enregistrement du même câble comme micro. Faderust coupe les boucles de rétroaction (une sortie joue sur le câble dont provient une entrée) et affiche « Boucle ».

### Faderust Kabel

Le Faderust Kabel est un périphérique audio USB virtuel (stéréo, 48 kHz, 24 bits) que Faderust fournit lui-même. Faderust l’affiche comme « Faderust Kabel (Lecture) » et « Faderust Kabel (Enregistrement) » ; ce qu’un programme joue sur le côté lecture ressort du côté enregistrement.

1. Sous **Appareils** → **Faderust Kabel**, cliquez sur **Installer** (une seule fois, avec une demande d’administrateur). Faderust télécharge [usbip-win2](https://github.com/vadimgrn/usbip-win2), vérifie la somme de contrôle et l’installe en arrière-plan. Il n’apparaît pas séparément dans la liste des applications Windows.
2. Activez **Faderust Kabel** et choisissez le nombre (1 à 4), par exemple un pour le micro et un pour les programmes. Windows les affiche dans ses paramètres comme « Faderust Cable », « Faderust Cable 2 », etc.
3. **Supprimer** annule l’installation, usbip-win2 compris.

Remarque : le câble est encore expérimental et a été testé sur peu de matériel.

## Limites

- Les programmes mémorisent faders, sorties et effets tant que leur tranche reste dans la table : si Discord redémarre, il revient avec ses anciens réglages. Si vous supprimez la tranche sous « Appareils », ils sont perdus.
- Pas de pilote noyau propre. Les câbles virtuels (« Faderust Kabel », jusqu’à quatre) sont des périphériques audio USB que Faderust fournit lui-même via USB/IP. Windows utilise pour cela son pilote audio USB intégré ; la partie USB/IP est [usbip-win2](https://github.com/vadimgrn/usbip-win2) (BSD-2, signé par Microsoft), que l’installation met en place une seule fois.
- Windows fournit le son par programme, pas par onglet de navigateur ou application web. Tout ce qui vient d’un navigateur passe par le même processus audio.
- Après une libération, le Bluetooth met environ une seconde avant que le son revienne. Pour l’éviter, réglez la sortie sur « Jamais libérer ».
- L’association « programme → appareil » utilise la même interface Windows non documentée que le mélangeur de volume des paramètres.

Réglages : `%APPDATA%\io.github.niklas970.faderust\config.toml`, à côté le journal `faderust.log`, utile en cas d’erreur.

## Développement

Rust pour le cœur audio et l’interface ([iced](https://iced.rs), sans WebView2).

| Chemin | Contenu |
|---|---|
| `crates/engine` | Cœur audio : appareils (WASAPI via cpal), tampons circulaires, compensation de dérive, matrice de mixage, effets, capture de programmes |
| `crates/usbcable` | Faderust Kabel : périphérique audio USB virtuel via USB/IP |
| `crates/core` | Logique derrière l’interface : configuration, appareils, programmes, câbles, télécommande, mise à jour |
| `crates/gui` | Le programme `faderust` : fenêtre iced, zone de notification, démarrage automatique |
| `streamdeck` | Plugin Stream Deck |

Prérequis : Rust (stable) ; Node.js pour le plugin Stream Deck.

```sh
cargo test
cargo run --release -p faderust
```

- Construire le programme d’installation : `makensis` avec `installer/faderust.nsi` (les paramètres figurent en tête du fichier). Les releases sont produites par le workflow « Release ».
- Moteur sans interface : `cargo run --release -p faderust-engine --example mixer -- --list`

Avant un commit : `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`. Les nouvelles dépendances nécessitent une licence permissive (`cargo deny check`).

## Stream Deck

Le plugin dans [`streamdeck/`](streamdeck/README.md) pilote Faderust depuis l’Elgato Stream Deck : faders par bouton rotatif ou touche, mute, presets et appareil par défaut de Windows. Double-cliquez sur `com.niklas970.faderust.streamDeckPlugin` depuis les [Releases](https://github.com/Niklas970/faderust/releases) (Stream Deck 7.1 ou plus récent). Ensuite, le programme d’installation fournit le plugin : sous « Appareils », Faderust indique si une version plus récente du plugin est disponible et l’ouvre d’un clic dans le logiciel Stream Deck, qui demande alors seulement une confirmation.

## Licence

[MIT](LICENSE)
