#include <assert.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

typedef struct {
  char name[16 + 1];
  unsigned char c1;
  unsigned char c2;
} WagerData;

typedef union {
  char name[16 + 1];
  WagerData wd;
} GameEventData;

typedef enum { PlayerJoined = 0, PlayerLeft = 1, Wager = 2 } GameEventType;

typedef struct {
  GameEventType type;
  GameEventData data;
} GameEvent;

typedef void (*CallbackFunc)(const GameEventData *);

typedef struct {
  CallbackFunc *con;
  size_t size;
  size_t capacity;
} SubscribersList;

void subscriberslist_init(SubscribersList *sl, size_t init_capacity) {
  sl->con = malloc(init_capacity * sizeof(GameEvent));
  sl->capacity = init_capacity;
  sl->size = 0;
}

void subscriberslist_realloc(SubscribersList *sl, size_t new_capacity) {
  assert(new_capacity > sl->capacity);
  sl->con = realloc(sl->con, new_capacity * sizeof(GameEvent));
  sl->capacity = new_capacity;
}

void subscriberslist_init_all(SubscribersList *sl, size_t init_capacity,
                              size_t size) {
  for (size_t i = 0; i < size; ++i) {
    subscriberslist_init(&sl[i], init_capacity);
  }
}

void subscriberslist_subscribe(SubscribersList *sl, GameEventType type,
                               CallbackFunc f) {
  SubscribersList *list = &sl[(int)type];
  assert(list->size < list->capacity);
  list->con[list->size++] = f;
}

void subscriberslist_unsubscribe(SubscribersList *sl, GameEventType type,
                                 CallbackFunc f) {
  SubscribersList *list = &sl[(int)type];
  assert(list->size > 0);
  if (list->size == 1) {
    list->con[0] = NULL;
    return;
  }
  for (size_t i = 0; i < list->capacity; ++i) {
    if (list->con[i] == f) {
      list->con[i] = list->con[list->size - 1];
      list->con[list->size - 1] = NULL;
      return;
    }
  }
  // Bad input. Cannot call unsubscribe on empty array
  assert(0);
}

void subscriberslist_emit(SubscribersList *sl, GameEvent *event) {
  SubscribersList *list = &sl[(int)event->type];
  for (size_t i = 0; i < list->size; ++i) {
    list->con[i](&event->data);
  }
}

void join(const GameEventData *data) { printf("%s joined :D\n", data->name); }
void join2(const GameEventData *data) {
  printf("%s joined :D but this is a different function\n", data->name);
}
void left(const GameEventData *data) { printf("%s left :(\n", data->name); }
void wager(const GameEventData *data) {
  printf("%s wagered %d %d's!\n", data->wd.name, data->wd.c1, data->wd.c2);
}

int main() {
  SubscribersList subscribers[3];

  subscriberslist_init_all(subscribers, 1, 3);

  subscriberslist_subscribe(subscribers, PlayerJoined, join);
  subscriberslist_realloc(&subscribers[(int)PlayerJoined], 2);
  subscriberslist_subscribe(subscribers, PlayerJoined, join2);
  subscriberslist_subscribe(subscribers, PlayerLeft, left);
  subscriberslist_subscribe(subscribers, Wager, wager);

  GameEvent event;

  event.type = PlayerJoined;
  strcpy(event.data.name, "Bob");
  subscriberslist_emit(subscribers, &event);

  event.type = PlayerLeft;
  strcpy(event.data.name, "Adam");
  subscriberslist_emit(subscribers, &event);

  event.type = Wager;
  strcpy(event.data.wd.name, "Tony");
  event.data.wd.c1 = 4;
  event.data.wd.c2 = 3;
  subscriberslist_emit(subscribers, &event);
}
