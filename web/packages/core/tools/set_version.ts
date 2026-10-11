import { replaceInFileSync } from "replace-in-file";
import childProcess from "child_process";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

// Ruffle Legaxy: versao propria (web/legaxy-version.json). O que aparece para o usuario
// (menu do clique direito, painel de informacoes, tela de pane) e "Ruffle Legaxy 1.0.1".
// A versao do Ruffle base (package.json) continua em %UPSTREAM_VERSION%, usada so na escolha
// entre fontes do Ruffle na pagina (extensao x selfhosted), para nao perder prioridade.
const __legaxyDir = path.dirname(fileURLToPath(import.meta.url));
const legaxy = JSON.parse(
    fs.readFileSync(
        path.resolve(__legaxyDir, "../../../legaxy-version.json"),
        "utf8",
    ),
) as { name: string; version: string; channel: string };
const upstreamVersion = process.env["npm_package_version"] ?? "";

let buildDate = new Date().toISOString();
let versionNumber = legaxy.version;
let versionChannel = process.env["CFG_RELEASE_CHANNEL"] || legaxy.channel;
const firefoxExtensionId =
    process.env["FIREFOX_EXTENSION_ID"] || "ruffle@ruffle.rs";

let commitHash = "unknown";

try {
    commitHash = childProcess.execSync("git rev-parse HEAD").toString().trim();
} catch {
    console.log("Couldn't fetch latest git commit...");
}

let versionName = `${legaxy.name} ${versionNumber}`;

interface VersionInformation {
    version_number: string;
    version_name: string;
    version_channel: string;
    build_date: string;
    commitHash: string;
    version4: string;
    firefox_extension_id: string;
}

let versionSeal: VersionInformation;

if (process.env["ENABLE_VERSION_SEAL"] === "true") {
    const __filename = fileURLToPath(import.meta.url);
    const __dirname = path.dirname(__filename);
    const sealFile = path.resolve(__dirname, "../../../version_seal.json");
    if (fs.existsSync(sealFile)) {
        console.log("Using version seal");
        // Using the version seal stored previously.
        versionSeal = JSON.parse(
            fs.readFileSync(sealFile, { encoding: "utf8" }),
        ) as VersionInformation;

        versionNumber = versionSeal.version_number;
        versionName = versionSeal.version_name;
        versionChannel = versionSeal.version_channel;
        buildDate = versionSeal.build_date;
        commitHash = versionSeal.commitHash;
    } else {
        console.log("Creating version seal");
        versionSeal = {
            version_number: versionNumber,
            version_name: versionName,
            version_channel: versionChannel,
            build_date: buildDate,
            commitHash: commitHash,
            version4: process.env["VERSION4"] ?? "",
            firefox_extension_id: firefoxExtensionId,
        };

        fs.writeFileSync(sealFile, JSON.stringify(versionSeal));
    }
}

const fallbackWasmName =
    process.env["BUILD_WASM_MVP"] === "true"
        ? "ruffle_web-wasm_mvp"
        : "ruffle_web";

const options = {
    files: "dist/**",
    from: [
        /%VERSION_NUMBER%/g,
        /%VERSION_NAME%/g,
        /%VERSION_CHANNEL%/g,
        /%BUILD_DATE%/g,
        /%COMMIT_HASH%/g,
        /%FALLBACK_WASM%/g,
        /%UPSTREAM_VERSION%/g,
    ],
    to: [
        versionNumber,
        versionName,
        versionChannel,
        buildDate,
        commitHash,
        fallbackWasmName,
        upstreamVersion,
    ],
};

replaceInFileSync(options);
