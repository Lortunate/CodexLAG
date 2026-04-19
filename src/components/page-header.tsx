export function PageHeader({
  eyebrow,
  titleId,
  title,
  description,
}: {
  eyebrow?: string;
  titleId?: string;
  title: string;
  description: string;
}) {
  return (
    <header className="page-header">
      {eyebrow ? <p className="page-header__eyebrow">{eyebrow}</p> : null}
      <h1 className="page-header__title" id={titleId}>
        {title}
      </h1>
      <p className="page-header__description">{description}</p>
    </header>
  );
}
