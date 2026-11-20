select "A", "a", "ab c", "SELECT", date, name, a as filter from t;
--     ^^^ needless-quotes
--               ^^^^^^ special-chars-in-identifier
--                                 ^^^^ keyword-identifier
--                                       ^^^^ keyword-identifier
--                                                  ^^^^^^ keyword-identifier
