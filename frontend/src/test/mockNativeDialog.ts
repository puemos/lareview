import { afterAll, beforeAll } from 'vitest';

export function mockNativeDialog() {
  const showModal = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'showModal');
  const close = Object.getOwnPropertyDescriptor(HTMLDialogElement.prototype, 'close');
  beforeAll(() => {
    Object.defineProperty(HTMLDialogElement.prototype, 'showModal', {
      configurable: true,
      value() {
        this.setAttribute('open', '');
      },
    });
    Object.defineProperty(HTMLDialogElement.prototype, 'close', {
      configurable: true,
      value() {
        this.removeAttribute('open');
      },
    });
  });
  afterAll(() => {
    if (showModal) Object.defineProperty(HTMLDialogElement.prototype, 'showModal', showModal);
    else delete (HTMLDialogElement.prototype as Partial<HTMLDialogElement>).showModal;
    if (close) Object.defineProperty(HTMLDialogElement.prototype, 'close', close);
    else delete (HTMLDialogElement.prototype as Partial<HTMLDialogElement>).close;
  });
}
