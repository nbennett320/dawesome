export enum DragEffectEnum {
  All = "all",
  Copy = "copy",
  CopyOrLink = "copyLink",
  CopyOrMove = "copyMove",
  Link = "link",
  LinkOrMove = "linkMove",
  Move = "move",
  None = "none",
}

export enum DropEffectEnum {
  Copy = "copy",
  Link = "link",
  Move = "move",
  None = "none",
}

export type DragEffect = DragEffectEnum | "all" | "move" | "copy" | "link" | "copyMove" | "copyLink" | "linkMove" | "none"
export type DropEffect = DropEffectEnum | "none" | "move" | "copy" | "link"
