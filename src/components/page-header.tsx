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
      <div className="page-header__body">
        {eyebrow ? <p className="page-header__eyebrow">{eyebrow}</p> : null}
        <div className="page-header__title-block">
          <h1 className="page-header__title" id={titleId}>
            {title}
          </h1>
          <p className="page-header__description">{description}</p>
        </div>
      </div>
    </header>
  );
}
