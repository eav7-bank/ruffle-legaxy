# Ruffle Legaxy 0.0.1

Versão própria do Ruffle para o **Naruto Legaxy** — https://naruto.legaxygames.com/play/ruffle-legaxy.html
(código: https://github.com/eav7-bank/ruffle-legaxy).

Baseado no **Ruffle** (https://ruffle.rs), licenças **MIT** e **Apache-2.0** — os textos estão em
`LICENSE_MIT` e `LICENSE_APACHE` neste pacote. Copyright (c) 2018 Ruffle LLC <ruffle@ruffle.rs> and Ruffle contributors.

O texto abaixo é o README original do pacote `ruffle-selfhosted`.

---

# ruffle-selfhosted

ruffle-selfhosted is the intended way to get Ruffle onto your website.

You may either include it and forget about it, and we will polyfill existing Flash content,
or use our APIs for custom configurations or more advanced usages of the Ruffle player.

## Using ruffle-selfhosted

For more examples and in-depth documentation on how to use Ruffle on your website, please
[check out our wiki](https://github.com/ruffle-rs/ruffle/wiki/Using-Ruffle#web).

### Host Ruffle

The `selfhosted` package is configured for websites that do not use bundlers or npm and just want
to get up and running. If you'd prefer to use Ruffle through npm and a bundler, please
[refer to ruffle core](https://github.com/ruffle-rs/ruffle/tree/master/web/packages/core).

Before you can get started with using Ruffle on your website, you must host its files yourself.
Either take the [latest build](https://github.com/ruffle-rs/ruffle/releases)
or [build it yourself](https://github.com/ruffle-rs/ruffle/blob/master/web/README.md), and make these files accessible by your web server.

Please note that the `.wasm` file must be served properly, and some web servers may not do that
correctly out of the box. Please see [our wiki](https://github.com/ruffle-rs/ruffle/wiki/Using-Ruffle#configure-wasm-mime-type)
for instructions on how to configure this, if you encounter a `Incorrect response MIME type` error.

### "Plug and Play"

If you have an existing website with flash content, you can simply include Ruffle as a script and
our polyfill magic will replace everything for you. No fuss, no mess.

```html
<script src="path/to/ruffle/ruffle.js"></script>
```

### Javascript API

If you want to control the Ruffle player, you may use our Javascript API.

```html
<script>
    window.RufflePlayer = window.RufflePlayer || {};

    window.addEventListener("DOMContentLoaded", () => {
        let ruffle = window.RufflePlayer.newest();
        let player = ruffle.createPlayer();
        let container = document.getElementById("container");
        container.appendChild(player);
        player.ruffle().load("movie.swf");
    });
</script>
<script src="path/to/ruffle/ruffle.js"></script>
```

## Building, testing or contributing

Please see [the ruffle-web README](https://github.com/ruffle-rs/ruffle/blob/master/web/README.md).
