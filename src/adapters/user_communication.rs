use business::usecase::{NewEntry, UpdateEntry};
use entities::Entry;

pub fn email_confirmation_email(u_id: &str) -> String {
    format!("Na du Weltverbesserer*,\nwir freuen uns dass du bei der Karte von Morgen mit dabei bist!\n\nBitte bestätige deine Email-Adresse hier:\nhttps://kartevonmorgen.org/#/?confirm_email={}.\n\neuphorische Grüße\ndas Karte von Morgen-Team",
    u_id)
}

pub fn new_entry_email(e: &NewEntry, id: &str, categories: Vec<String>) -> String {
    let intro_sentence = "ein neuer Eintrag auf der Karte von Morgen wurde erstellt";
    let entry = Entry {
        id : id.into(),
        title:e.title.clone(),
        description:e.description.clone(),
        street:e.street.clone(),
        zip:e.zip.clone(),
        city:e.city.clone(),
        country :e.country.clone(),
        email:e.email.clone(),
        telephone:e.telephone.clone(),
        homepage:e.homepage.clone(),
        categories:e.categories.clone(),
        lat:0.0,
        lng:0.0,
        created:0,
        version:0,
        license:None
    };
    entry_email(&entry, categories, &e.tags, intro_sentence)
}

pub fn changed_entry_email(e: &UpdateEntry, categories: Vec<String>) -> String {
    let intro_sentence = "folgender Eintrag der Karte von Morgen wurde verändert";
    let entry = Entry {
        id : e.id.clone(),
        title:e.title.clone(),
        description:e.description.clone(),
        street:e.street.clone(),
        zip:e.zip.clone(),
        city:e.city.clone(),
        country :e.country.clone(),
        email:e.email.clone(),
        telephone:e.telephone.clone(),
        homepage:e.homepage.clone(),
        categories:e.categories.clone(),
        lat:0.0,
        lng:0.0,
        created:0,
        version:0,
        license:None
    };
    entry_email(&entry, categories, &e.tags, intro_sentence)
}

pub fn entry_email(e: &Entry, categories: Vec<String>, tags: &Vec<String>, intro_sentence: &str) -> String{
    let category = if categories.len() > 0 { categories[0].clone() } else { "".to_string() };
    let address = vec![
        e.street.clone().unwrap_or("".into()),
        vec![e.zip.clone().unwrap_or("".into()),
            e.city.clone().unwrap_or("".into())].join(" "),
        e.country.clone().unwrap_or("".into())]
        .join(", ");

    format!(
"Hallo,
{introSentence}:\n
{title} ({category})
{description}\n
    Tags: {tags}
    Adresse: {address}
    Webseite: {homepage}
    Email-Adresse: {email}
    Telefon: {telephone}\n
Eintrag anschauen oder bearbeiten:
https://kartevonmorgen.org/#/?entry={id}\n
Du kannst dein Abonnement des Kartenbereichs abbestellen indem du dich auf https://kartevonmorgen.org einloggst.\n
euphorische Grüße
das Karte von Morgen-Team",
        introSentence = intro_sentence,
        title = &e.title, 
        id = &e.id,
        description= &e.description,
        address = address,
        email = e.email.clone().unwrap_or("".into()),
        telephone = e.telephone.clone().unwrap_or("".into()),
        homepage = e.homepage.clone().unwrap_or("".into()),
        category = category,
        tags = tags.join(", "))
}