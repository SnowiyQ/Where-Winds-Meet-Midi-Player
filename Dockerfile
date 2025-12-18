FROM node:20-bullseye

WORKDIR /app

COPY package*.json ./
RUN npm install

COPY . ./

ENV PORT=5173
ENV CHOKIDAR_USEPOLLING=true
EXPOSE 5173
CMD ["npm", "run", "dev", "--", "--host", "0.0.0.0", "--port", "5173"]
