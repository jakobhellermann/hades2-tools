import jsonview from "@pgrabovets/json-view";
import * as hades2 from "../rust-bindings/out/hades2_bindings.js";
import "./style.css";

let fileselect = document.getElementById("fileselect") as HTMLInputElement;
let errorText = document.getElementById("error") as HTMLParagraphElement;

let downloadTextBtn = document.getElementById("download-text") as HTMLButtonElement;
let downloadJsonBtn = document.getElementById("download-json") as HTMLButtonElement;
let loadJsonBtn = document.getElementById("load-json") as HTMLButtonElement;

let viewer = document.getElementById("viewer")!;

downloadTextBtn.addEventListener("click", () => data && processSavefile(data, "text"));
downloadJsonBtn.addEventListener("click", () => data && processSavefile(data, "json-pretty"));
loadJsonBtn.addEventListener("click", () => data && loadJson(data));

type Format = "json" | "json-pretty" | "text";

fileselect.addEventListener("change", onFileselectChange);
onFileselectChange();

let data: Uint8Array | null;

async function onFileselectChange() {
  errorText.innerText = "";
  let set = fileselect.files?.[0] != null;
  downloadTextBtn.disabled = !set;
  downloadJsonBtn.disabled = !set;
  loadJsonBtn.disabled = !set;

  let file = fileselect.files?.[0];
  if (!file) return;

  data = await loadFile(file);
}

function loadFile(file: File): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    let reader = new FileReader();
    reader.onerror = reject;
    reader.onloadend = (_) => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.readAsArrayBuffer(file);
  });
}

function expandSavefile(data: Uint8Array, format: Format): string | null {
  try {
    return hades2.expand_savefile(data, format);
  } catch (error) {
    let text: string;
    if (error instanceof Error) text = error.message;
    else if (typeof error == "string") text = error;
    else text = "" + error;

    errorText.innerText = `Could not parse savefile: ${text}`;
    return null;
  }
}

async function processSavefile(data: Uint8Array, format: Format) {
  errorText.innerText = "";
  let expanded = expandSavefile(data, format);
  if (!expanded) return;

  let url = URL.createObjectURL(new Blob([expanded]));
  let link = document.createElement("a");
  link.href = url;
  let extension = format == "text" ? "txt" : "json";
  link.download = `${fileselect.files?.[0]?.name}.${extension}`;
  link.click();
  URL.revokeObjectURL(url);
  link.remove();
}

function loadJson(data: Uint8Array) {
  let json = expandSavefile(data, "json");
  if (!json) return;

  let tree = jsonview.create(json);
  jsonview.render(tree, viewer);
}
