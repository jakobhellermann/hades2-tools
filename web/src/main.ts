import './style.css';
import * as hades2 from "../rust-bindings/out/hades2_bindings.js";

let fileselect = document.getElementById("fileselect") as HTMLInputElement;
let errorText = document.getElementById("error") as HTMLParagraphElement;

function loadFile(file: File): Promise<Uint8Array> {
  return new Promise((resolve, reject) => {
    let reader = new FileReader();
    reader.onerror = reject;
    reader.onloadend = _ => resolve(new Uint8Array(reader.result as ArrayBuffer));
    reader.readAsArrayBuffer(file);
  });
}

async function processSavefile() {
  let file = fileselect.files?.[0];
  if (!file) return;

  let data = await loadFile(file);
  let expanded: string;
  try {
    expanded = hades2.expand_savefile(data);
  } catch (error) {
    let text: string;
    if (error instanceof Error) text = error.message;
    else if (typeof error == "string") text = error;
    else text = "" + error;

    errorText.innerText = `Could not parse savefile: ${text}`;
    return;
  }
  console.log(expanded.length / 1024 / 1024);

  let url = URL.createObjectURL(new Blob([expanded]));
  let link = document.createElement("a");
  link.href = url;
  link.download = file.name + ".txt";
  link.click();
  URL.revokeObjectURL(url);
  link.remove();
}

fileselect.addEventListener("change", processSavefile);
// if (import.meta.env.DEV) processSavefile();