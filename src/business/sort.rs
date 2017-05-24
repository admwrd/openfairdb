use entities::*;
use std::cmp::Ordering;
use super::geo::{self, Coordinate};

trait DistanceTo {
    fn distance_to(&self, &Coordinate) -> f64;
}

impl DistanceTo for Entry {
    fn distance_to(&self, c: &Coordinate) -> f64 {
        geo::distance(&Coordinate {
                          lat: self.lat,
                          lng: self.lng,
                      },
                      c)
    }
}

pub trait SortByDistanceTo {
    fn sort_by_distance_to(&mut self, &Coordinate);
}

impl SortByDistanceTo for Vec<Entry> {
    fn sort_by_distance_to(&mut self, c: &Coordinate) {
        if !(c.lat.is_finite() && c.lng.is_finite()) {
            return;
        }
        self.sort_by(|a, _|
            if a.lat.is_finite() && a.lng.is_finite() {
                Ordering::Less
            } else {
                warn!("invalid coordinate: {}/{}", a.lat, a.lng);
                Ordering::Greater
            }
        );
        self.sort_by(|a, b| {
            a.distance_to(c).partial_cmp(&b.distance_to(c)).unwrap_or(Ordering::Equal)
        })
    }
}

pub trait Rated {
    fn average_rating(&self, &[Rating], &[Triple]) -> f64;
}

impl Rated for Entry {
    fn average_rating(&self, ratings: &[Rating], triples: &[Triple]) -> f64 {
        let entry_ratings : Vec<(&String, &String)> = triples
            .into_iter()
            .filter_map(|x| match *x {
                Triple {
                    subject   : ObjectId::Entry(ref e_id),
                    predicate : Relation::IsRatedWith,
                    object    : ObjectId::Rating(ref r_id)
                } => Some((e_id, r_id)),
                _ => None
            })
            .filter(|entry_rating| *entry_rating.0 == self.id).collect();

        let avg = ratings
            .into_iter()
            .filter_map(|rating| if entry_ratings.iter().any(|entry_rating| *entry_rating.1 == rating.id) { Some(rating) } else { None })
            .fold(0, |acc, ref rating| acc + rating.value) as f64
            / entry_ratings.len() as f64;

        if !avg.is_nan() { 
            avg as f64
        } else { 
            0.0
        }
    }
}

pub trait SortByAverageRating {
    fn sort_by_avg_rating(&mut self, &[Rating], &[Triple]);
}

impl SortByAverageRating for Vec<Entry> {
    fn sort_by_avg_rating(&mut self, ratings: &[Rating], triples: &[Triple]){
        self.sort_by(|a, b| {
            b.average_rating(ratings, triples)
            .partial_cmp(&a.average_rating(ratings, triples))
            .unwrap_or(Ordering::Equal)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn new_entry(id: &str, lat: f64, lng: f64) -> Entry { 
        Entry{
            id          : id.into(),
            created     : 0,
            version     : 0,
            title       : "foo".into(),
            description : "bar".into(),
            lat         : lat,
            lng         : lng,
            street      : None,
            zip         : None,
            city        : None,
            country     : None,
            email       : None,
            telephone   : None,
            homepage    : None,
            categories  : vec![],
            license     : None,
        }
    }

    fn new_rating(id: &str, value: i8) -> Rating {
        Rating{
            id         : id.into(),
            created    : 0,
            title      : "blubb".into(),
            value      : value.into(), 
            context    : RatingContext::Diversity
        }
    }

    #[test]
    fn test_average_rating() {
        let mut entry1 = new_entry("a", 0.0, 0.0);
        let mut entry2 = new_entry("b", 0.0, 0.0);
        let mut entry3 = new_entry("c", 0.0, 0.0);

        let ratings = vec![
            new_rating("1", 0),
            new_rating("2", 0),
            new_rating("3", 3),
            new_rating("4", 3),
            new_rating("5", -3),
            new_rating("6", 3),
        ];

        let triples = vec![
            Triple{subject: ObjectId::Entry("a".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("1".into())},
            Triple{subject: ObjectId::Entry("a".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("2".into())},
            Triple{subject: ObjectId::Entry("a".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("3".into())},
            Triple{subject: ObjectId::Entry("a".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("4".into())},
            Triple{subject: ObjectId::Entry("b".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("5".into())},
            Triple{subject: ObjectId::Entry("b".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("6".into())},
        ];

        assert_eq!(entry1.average_rating(&ratings, &triples), 1.5);
        assert_eq!(entry2.average_rating(&ratings, &triples), 0.0);
        assert_eq!(entry3.average_rating(&ratings, &triples), 0.0);
    }

    #[test]
    fn test_sort_by_avg_rating(){
        let mut entries = vec![
            new_entry("a", 0.0, 0.0),
            new_entry("b", 0.0, 0.0),
            new_entry("c", 0.0, 0.0),
            new_entry("d", 0.0, 0.0),
            new_entry("e", 0.0, 0.0),
        ];

        let ratings = vec![
            new_rating("1", 0),
            new_rating("2", 10),
            new_rating("3", 3),
            new_rating("4", -1),
            new_rating("5", 0),
        ];

        let triples = vec![
            Triple{subject: ObjectId::Entry("b".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("1".into())},
            Triple{subject: ObjectId::Entry("b".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("2".into())},
            Triple{subject: ObjectId::Entry("c".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("3".into())},
            Triple{subject: ObjectId::Entry("d".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("4".into())},
            Triple{subject: ObjectId::Entry("e".into()), predicate: Relation::IsRatedWith, object: ObjectId::Rating("5".into())},
        ];

        entries.sort_by_avg_rating(&ratings, &triples);


        assert_eq!(entries[0].id, "b");
        assert_eq!(entries[1].id, "c");
        assert!(entries[2].id == "a" || entries[2].id == "e");
        assert!(entries[3].id == "a" || entries[3].id == "e");
        assert_eq!(entries[4].id, "d");


        // tests:
        // - negative ratings
    }

    #[test]
    fn sort_by_distance() {
        let mut entries = vec![new_entry("a", 1.0, 0.0),
                               new_entry("b", 0.0, 0.0),
                               new_entry("c", 1.0, 1.0),
                               new_entry("d", 0.0, 0.5),
                               new_entry("e", -1.0, -1.0)];
        let x = Coordinate {
            lat: 0.0,
            lng: 0.0,
        };
        entries.sort_by_distance_to(&x);
        assert_eq!(entries[0].id, "b");
        assert_eq!(entries[1].id, "d");
        assert_eq!(entries[2].id, "a");
        assert!(entries[3].id == "c" || entries[3].id == "e");
        assert!(entries[4].id == "c" || entries[4].id == "e");
    }

    use std::f64::{NAN, INFINITY};

    #[test]
    fn sort_with_invalid_coordinates() {
        let mut entries = vec![new_entry("a", 1.0, NAN),
                               new_entry("b", 1.0, INFINITY),
                               new_entry("c", 2.0, 0.0),
                               new_entry("d", NAN, NAN),
                               new_entry("e", 1.0, 0.0)];
        let x = Coordinate {
            lat: 0.0,
            lng: 0.0,
        };
        entries.sort_by_distance_to(&x);
        assert_eq!(entries[0].id, "e");
        assert_eq!(entries[1].id, "c");

        let mut entries =
            vec![new_entry("a", 2.0, 0.0), new_entry("b", 0.0, 0.0), new_entry("c", 1.0, 0.0)];

        let x = Coordinate {
            lat: NAN,
            lng: 0.0,
        };
        entries.sort_by_distance_to(&x);
        assert_eq!(entries[0].id, "a");
        assert_eq!(entries[1].id, "b");
        assert_eq!(entries[2].id, "c");
    }

}
