import { mount } from "svelte";
import "./app.css";
import App from "./App.svelte";
import { applyTheme, loadTheme } from "./lib/theme.js";

applyTheme(loadTheme());

export default mount(App, { target: document.getElementById("app") });
