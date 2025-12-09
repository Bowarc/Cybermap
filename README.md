# Cybermap - A Cyberpunk2077 themed map web & mobile app  

## Goal  
Learn about mobile apps, dioxus and warp  

## Status  
Working build system for Web and Android(arm64 and amd64)  

IOS is not planned for now, I don't use any apple product so I don't really care atm  

The app is just the default dioxus interface  

## Stack
Rust only if possible (ofc java gradle for the apk but im not writing it)  

Web and mobile apps will be made using [Dioxus](https://dioxuslabs.com/)  

Webserver - [Warp](https://docs.rs/warp/latest/warp/) if I can, else i'll go back to [Rocket](https://rocket.rs/)  

## Roadmap  
- [ ] Webserver - I'm probably gonna use warp for it  
   - [ ] Serving the webapp  
   - [ ] ?proxy for anonymising map queries  

- [ ] UI
   - [ ] Map display at startup  
   - [ ] The ability to move around  
   - [ ] Zooms  
   - [ ] Cyberpunk-like icons for points of interest  
   - [ ] Pathfinding ?  

- [ ] Mobile  
   - [ ] UI implementation  

- [ ] Web  
   - [ ] UI implementation  
   - [ ] The ability to download the mobile apps  

## Installation  
TODO  
Hopefully it will be possible through the webapp or github releases  
