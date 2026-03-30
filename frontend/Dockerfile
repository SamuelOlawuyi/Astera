FROM node:20-alpine

WORKDIR /app

COPY package*.json ./
RUN npm install

# Rest is mapped via volume, but we still copy for completeness
COPY . .

EXPOSE 3000

CMD ["npm", "run", "dev"]
