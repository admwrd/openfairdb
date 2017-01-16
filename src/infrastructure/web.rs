use nickel::{Nickel, JsonBody, MediaType, QueryString, Request, Response, MiddlewareResult,
             HttpRouter};
use nickel::status::StatusCode;
use hyper::header::{AccessControlAllowOrigin, AccessControlAllowHeaders, AccessControlAllowMethods};
use hyper::method::Method;
use r2d2_cypher::CypherConnectionManager;
use r2d2::PooledConnection;
use adapters::json::{Entry, Category, SearchResult};
use business::repo::Repo;
use adapters::validate::Validate;
use business::error::{Error, ParameterError};
use infrastructure::error::{AppError, StoreError};
use rustc_serialize::json::encode;
use business::filter::{FilterByCategoryIds, FilterByBoundingBox};
use business::sort::SortByDistanceTo;
use r2d2;
use business::{search, geo};
use business::search::Search;
use entities;
use std::convert::TryFrom;

static POOL_SIZE: u32 = 5;
static MAX_INVISIBLE_RESULTS: usize = 5;

#[derive(Debug, Clone)]
struct Data {
    db: r2d2::Pool<CypherConnectionManager>,
}

impl Data {
    fn db_pool(&self) -> Result<PooledConnection<CypherConnectionManager>, AppError> {
        self.db
            .get()
            .map_err(StoreError::Pool)
            .map_err(AppError::Store)
    }
}

fn cors_middleware<'mw>(_req: &mut Request<Data>,
                        mut res: Response<'mw, Data>)
                        -> MiddlewareResult<'mw, Data> {
    res.set(AccessControlAllowOrigin::Any);
    res.set(AccessControlAllowHeaders(vec!["Origin".into(),
                                           "X-Requested-With".into(),
                                           "Content-Type".into(),
                                           "Accept".into()]));

    res.next_middleware()
}

fn extract_ids(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.to_owned())
        .filter(|id| id != "")
        .collect::<Vec<String>>()
}

#[test]
fn extract_ids_test() {
    assert_eq!(extract_ids("abc"), vec!["abc"]);
    assert_eq!(extract_ids("a,b,c"), vec!["a", "b", "c"]);
    assert_eq!(extract_ids("").len(), 0);
    assert_eq!(extract_ids("abc,,d"), vec!["abc", "d"]);
}

pub fn run(db_url: &str, port: u16, enable_cors: bool) {
    let manager = CypherConnectionManager { url: db_url.into() };
    let config = r2d2::Config::builder().pool_size(POOL_SIZE).build();
    let pool = r2d2::Pool::new(config, manager).unwrap();
    let data = Data { db: pool };

    let mut server = Nickel::with_data(data);

    if enable_cors {
        server.utilize(cors_middleware);
        server.options("/entries/*",
                       middleware!{|_, mut res|
                res.set(AccessControlAllowHeaders(vec!["Content-Type".into()]));
                res.set(AccessControlAllowMethods(vec![Method::Get, Method::Post, Method::Put]));
                StatusCode::Ok
            });
    }

    server.utilize(router! {

            get "/entries/:id" => |req, mut res|{
              match req.param("id")
                .ok_or(ParameterError::Id)
                    .map_err(Error::Parameter)
                    .map_err(AppError::Business)
                .and_then(|s|{
                  let ids = extract_ids(s);
                  let data: &Data = res.server_data();
                  data.db_pool()
                    .and_then(|ref pool|{

                      Entry::all(pool)
                        .map_err(AppError::Store)
                        .and_then(|entries|

                        match ids.len() {
                          0 => encode(&entries)
                            .map_err(Error::Encode)
                            .map_err(AppError::Business),
                          1 => entries
                              .iter()
                              .find(|x| x.id == Some(ids[0].clone()))
                              .ok_or(StoreError::NotFound).map_err(AppError::Store)
                              .and_then(|e| encode(&e)
                                .map_err(Error::Encode)
                                .map_err(AppError::Business)
                              ),
                          _ => encode(&entries
                                .iter()
                                .filter(|e| e.id.is_some())
                                .filter(|e| ids.iter().any(|id| e.id == Some(id.clone())))
                                .collect::<Vec<&Entry>>())
                                    .map_err(Error::Encode)
                                    .map_err(AppError::Business)
                        }
                      )
                    })
                })
                {
                  Ok(x)  => {
                    res.set(MediaType::Json);
                    (StatusCode::Ok, x)
                  },
                  Err(ref err) =>
                    (err.into(), format!("Could not fetch entries: {}", err))
                }
              }

            get "/duplicates/" => |_, mut res|{
              let data: &Data = res.server_data();
              match data.db_pool()
                .and_then(|ref pool|
                    {
                      let entries = Entry::all(pool).map_err(AppError::Store)?;
                      let entries = entries
                        .into_iter()
                        .map(entities::Entry::try_from)
                        //TODO: warn on error
                        .filter_map(|x|x.ok())
                        .collect();
                      let ids = search::find_duplicates(&entries);
                      encode(&ids).map_err(Error::Encode).map_err(AppError::Business)
                    }
                )
              {
                  Ok(r) => {
                    res.set(MediaType::Json);
                    (StatusCode::Ok, r)
                  },
                  Err(ref err) => {
                     (err.into(), format!("Error while trying to find duplicates: {}", err))
                  }
              }
            }

            post "/entries/:id" => |req, res|{
              match req.json_as::<Entry>()
                    .map_err(Error::Io)
                    .map_err(AppError::Business)
                .and_then(|json|{
                  try!(json.validate()
                    .map_err(Error::Validation)
                    .map_err(AppError::Business));
                  let data: &Data = res.server_data();
                  data.db_pool()
                    .and_then(|ref pool|
                      json.save(pool)
                        .map_err(AppError::Store)
                        .and_then(|e| e.id
                           .ok_or(StoreError::Save).map_err(AppError::Store))

                  )
              })
              {
                Ok(id)  => (StatusCode::Ok, id),
                Err(ref err) =>
                  (err.into(), format!("Could not create entry: {}", err))
              }
            }

            put "/entries/:id" => |req, res|{
              let entry = req.json_as::<Entry>();
              let data: &Data = res.server_data();
              match req.param("id")
                .ok_or(ParameterError::Id)
                    .map_err(Error::Parameter)
                    .map_err(AppError::Business)
                .and_then(|id| entry
                    .map_err(Error::Io)
                    .map_err(AppError::Business)
                  .and_then(|mut new_data|{
                    data.db_pool().and_then(|ref pool|
                      Entry::get(pool, id.to_owned())
                      .map_err(AppError::Store)
                      .and_then(|_|{
                        new_data.id = Some(id.to_owned());
                        new_data.save(pool)
                        .map_err(AppError::Store)
                        .and_then(|_| Ok(id.to_owned()))
                      })
                    )
                  })
                )
              {
                Ok(id) => {
                  (StatusCode::Ok, id)
                }
                Err(ref err) => {
                  let msg = format!("Could not save entry: {}", err);
                  warn!("{}", msg);
                  (err.into(), format!("Could not save entry: {}", err))
                }
              }
            }

            get "/categories/:id" => |req, mut res|{
              match req.param("id")
                .ok_or(ParameterError::Id)
                    .map_err(Error::Parameter)
                    .map_err(AppError::Business)
                .and_then(|s|{
                  let ids = extract_ids(s);
                  let data: &Data = res.server_data();
                  data.db_pool().and_then(|ref pool|{
                      Category::all(pool)
                        .map_err(AppError::Store)
                        .and_then(|categories|
                          match ids.len() {
                            0 => encode(&categories)
                                .map_err(Error::Encode)
                                .map_err(AppError::Business),
                            1 => {
                                let id = Some(ids[0].clone());
                                categories
                                  .iter()
                                  .find(|x| x.id == id)
                                  .ok_or(StoreError::NotFound).map_err(AppError::Store)
                                  .and_then(|e| encode(&e)
                                    .map_err(Error::Encode)
                                    .map_err(AppError::Business))
                            },
                            _ =>
                                encode(&categories.iter()
                                    .filter(|e| e.id.is_some())
                                    .filter(|e| ids.iter().any(|id| e.id == Some(id.clone())))
                                    .collect::<Vec<&Category>>())
                                        .map_err(Error::Encode)
                                        .map_err(AppError::Business)
                          }
                      )
                  })
                })
                {
                  Ok(x)  => {
                    res.set(MediaType::Json);
                    (StatusCode::Ok, x)
                  },
                  Err(ref err) =>
                    (err.into(), format!("Could not fetch categories: {}", err))
                }
              }

            get "/search" => |req, mut res| {
              let data: &Data = res.server_data();
              let query = req.query();
              match query
                .get("bbox")
                .ok_or(ParameterError::Bbox)
                    .map_err(Error::Parameter)
                    .map_err(AppError::Business)
                .and_then(|bbox_str| query
                .get("categories")
                .ok_or(ParameterError::Categories)
                    .map_err(Error::Parameter)
                    .map_err(AppError::Business)
                .and_then(|cat_str| data.db_pool()
                    .and_then(|ref pool|
                      Entry::all(pool)
                      .map_err(AppError::Store)
                      .and_then(|entries|{

                        let cat_ids = extract_ids(cat_str);
                        //TODO: get rid of unrwap
                        let bbox = geo::extract_bbox(bbox_str).unwrap();
                        let bbox_center = geo::center(&bbox[0], &bbox[1]);
                        let entries : Vec<entities::Entry>= entries
                            .into_iter()
                            .map(entities::Entry::try_from)
                            .filter_map(|x|x.ok())
                            .collect();
                        let cat_filtered_entries : Vec<&entities::Entry> =
                            entries.filter_by_category_ids(&cat_ids);

                        let pre_filtered_entries = match query.get("text") {
                          Some(txt) => cat_filtered_entries.filter_by_search_text(&txt.to_owned()),
                          None      => cat_filtered_entries
                        };

                        let mut pre_filtered_entries = pre_filtered_entries
                            .into_iter()
                            .cloned()
                            .collect::<Vec<entities::Entry>>();

                        pre_filtered_entries.sort_by_distance_to(&bbox_center);

                        let visible_results = pre_filtered_entries.filter_by_bounding_box(&bbox);

                        let invisible_results = pre_filtered_entries
                          .iter()
                          .filter(|e| !visible_results.iter().any(|&v| v.id == e.id ))
                          .take(MAX_INVISIBLE_RESULTS)
                          .collect::<Vec<_>>();

                        let search_result = SearchResult{
                          visible   : visible_results.into_iter().map(|x|x.id.clone()).collect(),
                          invisible : invisible_results.into_iter().map(|x|x.id.clone()).collect()
                        };

                        encode(&search_result)
                            .map_err(Error::Encode)
                            .map_err(AppError::Business)
                      })
                    )
              ))
              {
                Ok(x)  => {
                  res.set(MediaType::Json);
                  (StatusCode::Ok, x)
                },
                Err(ref err) =>
                  (err.into(), format!("Could not search entries: {}", err))
              }
          }

          get "/count/entries" => |_, res| {

              let data: &Data = res.server_data();
              match data.db_pool()
                .and_then(|ref pool| Entry::all(pool)
                      .map_err(AppError::Store)
                      .and_then(|entries| Ok(entries.into_iter().count().to_string())))
              {
                Ok(x)  => {
                  (StatusCode::Ok, x)
                },
                Err(ref err) =>
                  (err.into(), format!("Could not count entries: {}", err))
              }
            }

            get "/server/version" => { env!("CARGO_PKG_VERSION") }

        });

    server.listen(("127.0.0.1", port));
}
