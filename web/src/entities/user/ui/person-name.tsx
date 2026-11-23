type PersonNameProps = { displayName: string; username: string };

export function PersonName({ displayName, username }: PersonNameProps) {
  return (
    <span className="min-w-0">
      <span className="font-medium">{displayName}</span>{" "}
      <span className="text-muted-foreground">@{username}</span>
    </span>
  );
}
