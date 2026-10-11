# Ruffle Legaxy

**Versão própria do Ruffle para o Naruto Legaxy — versão 1.0.1.**

Página: https://naruto.legaxygames.com/play/ruffle-legaxy.html

O Ruffle Legaxy é o "Flash" que roda o Naruto Legaxy no navegador, sem instalar nada. É um fork do
[Ruffle](https://ruffle.rs) (emulador de Flash Player em Rust + WebAssembly) com correções feitas para o jogo.

## O que a versão 1.0.1 melhora
- **Textos que se movem não são redesenhados**: mover um campo de texto (nomes dos personagens, que o jogo
  reposiciona a cada quadro) não invalida mais o cache de bitmap dele — os nomes com brilho deixavam de ser
  redesenhados com filtro todo quadro. No celular o desenho caiu de 26 ms para 6,4 ms por quadro (30 fps).

## O que a versão 0.0.1 trouxe
- **Sem pane de memória nas lutas**: no Ruffle oficial a memória WebAssembly subia a cada quadro de efeito
  das lutas pesadas até o limite de 4 GiB (pane "unreachable"). Aqui isso não acumula mais.
- **Imagens compartilhadas**: cada BitmapData de símbolo da biblioteca é decodificado uma vez; as instâncias
  dividem os mesmos pixels e a mesma textura na GPU (copy-on-write: só copia se o AS3 escrever).
- **GC por bytes**: a dívida do coletor de lixo é proporcional aos bytes de pixels, não só ao nº de objetos.
- **Memória real**: `System.totalMemory`/`privateMemory` reais e coleta contínua do GC.
- **Medição**: heap vivo, BitmapData, texturas e quadros executados expostos ao JS (telemetria do jogo).
- **Identidade própria**: "Ruffle Legaxy" com a versão no clique direito ("Sobre o Ruffle Legaxy (Public 1.0.1)"), na tela de
  erro e nas informações de depuração; todos os links levam para a nossa página.

## Versão
- A versão fica em [`web/legaxy-version.json`](web/legaxy-version.json) (`name`, `version`, `channel`).
  O `web/packages/core/tools/set_version.ts` gera `versionName = "Ruffle Legaxy 1.0.1"`, canal `legaxy`.
- A versão do Ruffle oficial em que nos baseamos fica em `buildInfo.upstreamVersion` (hoje 0.7.1, master
  `fdf3d4db`); ela é usada só para a escolha entre fontes do Ruffle na página (extensão × selfhosted).
- Para lançar uma versão nova: mude `web/legaxy-version.json`, faça commit e crie a tag `vX.Y.Z`.

## Como compilar (máquina do Jonathan, Windows)
Ferramentas: Rust 1.99 + alvo `wasm32-unknown-unknown` + `rust-src`, Node 24, `wasm-bindgen-cli` 0.2.127,
Binaryen 133 (`wasm-opt`, em `%LOCALAPPDATA%\binaryen\binaryen-version_133\bin`), Java 21 (Temurin),
LLVM/clang (`C:\Program Files\LLVM\bin`, para a dependência C do jpegxr) e VS 2022 Build Tools (C++).

```powershell
$env:Path = "C:\Program Files\nodejs;$env:USERPROFILE\.cargo\bin;$env:LOCALAPPDATA\binaryen\binaryen-version_133\bin;C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot\bin;C:\Program Files\LLVM\bin;" + $env:Path
$env:JAVA_HOME = "C:\Program Files\Eclipse Adoptium\jdk-21.0.12.101-hotspot"
cd web
npm ci
npm run build:dual-wasm
```

Saída: `web/packages/selfhosted/dist/` — `ruffle.js` + 2 `.wasm` (com extensões/SIMD e o básico "mvp")
+ `LICENSE_MIT`, `LICENSE_APACHE`, `README.md`, `package.json`. No jogo, a pasta vai para
`client/www/ruffle_legaxy/` (sem o `ruffle.js.map`) e a `client/www/index.html` aponta o mapa `WASM` para o
nome do `.wasm` com extensões.

## Atribuição e licenças
O Ruffle Legaxy é **baseado no Ruffle (https://ruffle.rs), licenças MIT e Apache-2.0**.
Copyright (c) 2018 Ruffle LLC <ruffle@ruffle.rs> and Ruffle contributors. Os avisos de copyright do projeto
original foram mantidos; as alterações do Naruto Legaxy seguem as mesmas licenças (MIT OR Apache-2.0).

- Licenças do projeto: [`LICENSE.md`](LICENSE.md); no pacote web: `web/packages/selfhosted/LICENSE_MIT` e
  `web/packages/selfhosted/LICENSE_APACHE` (copiados para o `dist`).
- README original do Ruffle: [`README_RUFFLE.md`](README_RUFFLE.md).
- Projeto original: https://github.com/ruffle-rs/ruffle
