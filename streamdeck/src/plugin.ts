import streamDeck from "@elgato/streamdeck";

import { Device } from "./actions/device";
import { Mute } from "./actions/mute";
import { Preset } from "./actions/preset";
import { Volume } from "./actions/volume";
import { faderust } from "./connection";

faderust.on("error", (message) => streamDeck.logger.warn(`Faderust: ${message}`));

streamDeck.actions.registerAction(new Volume());
streamDeck.actions.registerAction(new Mute());
streamDeck.actions.registerAction(new Preset());
streamDeck.actions.registerAction(new Device());

faderust.version = streamDeck.info.plugin.version;
faderust.connect();
streamDeck.connect();
