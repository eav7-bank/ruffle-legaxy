/**
 * @returns The HTMLElement containing the hardware acceleration modal
 */
export function HardwareAcceleration() {
    return (
        <div id="hardware-acceleration-modal" class="modal hidden">
            <div class="modal-area">
                <span class="close-modal"></span>
                <span
                    id="acceleration-text"
                    data-i18n-key="enable-hardware-acceleration"
                ></span>
                <a
                    href="https://naruto.legaxygames.com/play/ruffle-legaxy.html#aceleracao"
                    target="_blank"
                    class="modal-button"
                    data-i18n-key="enable-hardware-acceleration-link"
                ></a>
            </div>
        </div>
    );
}
