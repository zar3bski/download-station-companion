# download-station-companion
Handle your downloads from a discord channel 


## Usage

**DS-companion** is designed to be executed as a **cronjob**

> currently, the only supported interval is every two minutes (`*/2 * * * *`)

### Set up a discord application/bot to provide an API access 

The bot: 
* does not need to be public
* needs the following permissions:
  * `View Channels`
  * `Send Messages`
  * `Read Message History`
  * `Add Reactions`
