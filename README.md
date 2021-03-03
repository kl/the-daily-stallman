# The Daily Stallman
Read the news like Stallman would. No JavaScript required.

The Daily Stallman reads the stallman.org RSS news feed, downloads articles and merges them into 
a single HTML file.

![example showing article text](./resources/example.png)

## Install
```cargo install the-daily-stallman```

## Usage
To run the-daily-stallman and open HTML file in Firefox, run:
```
tds && firefox tds.html
```

Use the `-o` flag to change where the HTML is written to:
```
tds -o ~/news.html
```

By default only the articles for the current day are downloaded. Yesterday's articles can be downloaded with 
the `--yesterday` flag:
```
tds --yesterday
```

## TODO
* Enable full offline reading by downloading article images.
* Add feature to output epub/mobi instead of HTML for reading on e-readers.
