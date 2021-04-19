String enumToString<T>(T enumItem, {bool camelCase = false}) {
  return enumItem.toString().split('.')[1];
}

T enumFromString<T>(
  List<T> enumValues,
  String value,
  T defaultValue,
) {
  // ignore: unnecessary_cast
  return enumValues.singleWhere(
      (enumItem) => enumToString(enumItem).toLowerCase() == value.toLowerCase(),
      orElse: () => defaultValue);
}
