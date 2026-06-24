// Stub annotations for fixture parsing (no Spring dependency required).
@interface GetMapping {
    String value() default "";
}

@interface PostMapping {
    String value() default "";
}

class Api {
    @GetMapping("/items")
    public void listItems() {}

    @PostMapping("/items")
    public void createItem() {}
}
