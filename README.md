# Cybermap - A Cyberpunk2077 themed map web & mobile app  

## Goal  
Learn about mobile apps, dioxus and warp  

## Status  
Working build system for Web and Android(arm64 and amd64)  

IOS is not planned for now, I don't use any apple product so I don't really care atm  

The web app is able to display osm roads and buildings in the style of cyberpunk's map*  
Also got a mobile app (for android) that has the same capabilities  

Got a proxy server over OSM to fetch the data and rate limit across users  
The server also serves the webapp  

*The map display needs a bit of rework but it's fine for now

## Stack
Rust only if possible (ofc java gradle for the apk but im not writing it)  

Web and mobile apps will be made using [Dioxus](https://dioxuslabs.com/)  

Webserver - [Warp](https://docs.rs/warp/latest/warp/) if I can, else i'll go back to [Rocket](https://rocket.rs/)  

## Roadmap  
- [x] Webserver  
   - [x] Serving the webapp  
   - [x] proxy for anonymising map queries
   - [x] Map webapp endpoints to /
- [ ] UI
   - [ ] Map display
      - [ ] Roads
         - [x] Outlines
         - [ ] Dynamic width depending on the road type
         - [ ] Merge roads that use the same nodes
      - [ ] Buildings
         - [x] Basic
         - [ ] 3D ?
      - [ ] Terain
      - [ ] Water
   - [ ] Cyberpunk-like icons for points of interest  
   - [ ] Settings ?
   - [ ] Pathfinding ?
- [ ] Mobile
   - [ ] UI implementation  
   - [ ] Geolocalisation
   - [x] Retrieving OSM data from server
   - [ ] Move around  
- [ ] Web  
   - [ ] UI implementation
   - [x] Geolocalisation
   - [x] Retrieving OSM data from server
   - [x] React to window resizing
   - [ ] Move around  
   - [ ] Download the mobile apps
- [x] OSM
   - [x] Parse OSM api response
   - [x] OSM types (nwr)
   - [x] Geographical types (Position, Boxes)
   

## Installation  
Still a big TODO  
Hopefully it will be possible through the webapp or github releases  
