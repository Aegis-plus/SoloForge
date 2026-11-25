import { render } from "solid-js/web";
import "@fontsource/alegreya/400.css";
import "@fontsource/alegreya/700.css";
import "@fontsource/alegreya/400-italic.css";
import "@fontsource/alegreya-sans/400.css";
import "@fontsource/alegreya-sans/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/700.css";
import "./styles/tokens.css";
import "./styles/app.css";
import { App } from "./App";
import { boot } from "./lib/store";

void boot().finally(() => render(() => <App />, document.getElementById("root")!));
