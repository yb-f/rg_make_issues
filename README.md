# rg_make_issues

Create issues on github from posts on rg forums.

## Usage

Compile, run.

## .env

You will need to create a .env file in the root directory with the following variables being set:

GH_AUTH_TOKEN = [your github auth token]
GH_OWNER = [your github username]
GH_REPO = [your github repo]
THREAD_ID = [the thread id you want to read]
API_KEY = [your rg api key]
API_USER_ID = [your rg api user id]
BASE_URL = [the base url of the rg forums]
USERNAME = [your username on rg forums]

For example:
GH_AUTH_TOKEN="ghp_1234567890"
GH_OWNER="rg"
GH_REPO="rg-forums"
THREAD_ID=123456
API_KEY="123abc456"
API_USER_ID="red: paco"
BASE_URL="https://www.redguides.com/community/api/"
USERNAME="paco"
