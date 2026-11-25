import { WidgetType } from "@codemirror/view";

const copiedResetMs = 1200;

export class BulletWidget extends WidgetType {
  override eq(): boolean {
    return true;
  }

  override toDOM(): HTMLElement {
    const bullet = document.createElement("span");
    bullet.className = "cm-bullet";
    bullet.textContent = "•";
    return bullet;
  }
}

export class CheckboxWidget extends WidgetType {
  readonly isChecked: boolean;
  readonly isEditable: boolean;

  constructor(isChecked: boolean, isEditable: boolean) {
    super();
    this.isChecked = isChecked;
    this.isEditable = isEditable;
  }

  override eq(other: CheckboxWidget): boolean {
    return other.isChecked === this.isChecked && other.isEditable === this.isEditable;
  }

  override toDOM(): HTMLElement {
    const box = document.createElement("input");
    box.type = "checkbox";
    box.checked = this.isChecked;
    box.disabled = !this.isEditable;
    box.className = "cm-task-checkbox";
    box.setAttribute("aria-label", this.isChecked ? "Checked item" : "Unchecked item");
    return box;
  }

  override ignoreEvent(): boolean {
    return false;
  }
}

export class RuleWidget extends WidgetType {
  override eq(): boolean {
    return true;
  }

  override toDOM(): HTMLElement {
    const rule = document.createElement("span");
    rule.className = "cm-rule";
    return rule;
  }
}

export class CodeHeaderWidget extends WidgetType {
  readonly language: string;
  readonly code: string;

  constructor(language: string, code: string) {
    super();
    this.language = language;
    this.code = code;
  }

  override eq(other: CodeHeaderWidget): boolean {
    return other.language === this.language && other.code === this.code;
  }

  override toDOM(): HTMLElement {
    const header = document.createElement("span");
    header.className = "cm-code-header";
    const label = document.createElement("span");
    label.className = "cm-code-language";
    label.textContent = this.language || "Plain text";
    const copy = document.createElement("button");
    copy.type = "button";
    copy.className = "cm-code-copy";
    copy.textContent = "Copy";
    copy.addEventListener("mousedown", (event) => event.preventDefault());
    copy.addEventListener("click", () => void this.copyTo(copy));
    header.append(label, copy);
    return header;
  }

  override ignoreEvent(event: Event): boolean {
    return event.target instanceof HTMLButtonElement;
  }

  private async copyTo(button: HTMLButtonElement) {
    await navigator.clipboard.writeText(this.code);
    button.textContent = "Copied";
    window.setTimeout(() => {
      button.textContent = "Copy";
    }, copiedResetMs);
  }
}

export class HintWidget extends WidgetType {
  readonly text: string;

  constructor(text: string) {
    super();
    this.text = text;
  }

  override eq(other: HintWidget): boolean {
    return other.text === this.text;
  }

  override toDOM(): HTMLElement {
    const hint = document.createElement("span");
    hint.className = "cm-line-hint";
    hint.setAttribute("aria-hidden", "true");
    hint.textContent = this.text;
    return hint;
  }
}
