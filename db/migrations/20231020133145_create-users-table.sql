CREATE TABLE User (
    ID INT NOT NULL PRIMARY KEY AUTO_INCREMENT,
    Username VARCHAR(20) NOT NULL,
    Password VARCHAR(255) NOT NULL,
    EloPoints INT NOT NULL DEFAULT 500,
    CountryID VARCHAR(3) NOT NULL DEFAULT "ID",
    ProfilePictureURL VARCHAR(1024) NOT NULL DEFAULT ""
)
