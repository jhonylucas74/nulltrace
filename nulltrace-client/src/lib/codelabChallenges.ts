/**
 * Codelab challenge definitions.
 * Each challenge has 3 test cases; the user must pass all 3 to complete it.
 */

export type Difficulty = "easy" | "medium" | "hard";

export interface TestCase {
  id: number;
  /** Human-readable description shown in the UI */
  input: string;
  /** Exact expected print output (newline-separated) */
  expectedOutput: string;
  /** Luau code appended after user code to invoke their function */
  setupCode: string;
}

export interface Challenge {
  id: string;
  title: string;
  difficulty: Difficulty;
  /** Problem statement shown on the left panel */
  description: string;
  /** Input/output examples shown below the description */
  examples: string;
  /** Initial code placed in the editor */
  starterCode: string;
  /** Exactly 3 test cases */
  testCases: [TestCase, TestCase, TestCase];
}

export const CHALLENGES: Challenge[] = [
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  EASY  (14 challenges)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  {
    id: "hello-world",
    title: "Hello World",
    difficulty: "easy",
    description:
      'Write a function `hello()` that prints "Hello, World!" to the console.',
    examples: "-- Expected output:\nHello, World!",
    starterCode: `function hello()
  -- your code here
end`,
    testCases: [
      { id: 1, input: "hello()", expectedOutput: "Hello, World!", setupCode: "hello()" },
      { id: 2, input: "hello() x2", expectedOutput: "Hello, World!\nHello, World!", setupCode: "hello()\nhello()" },
      { id: 3, input: "hello() x3", expectedOutput: "Hello, World!\nHello, World!\nHello, World!", setupCode: "hello()\nhello()\nhello()" },
    ],
  },
  {
    id: "sum-two",
    title: "Sum Two Numbers",
    difficulty: "easy",
    description: "Write a function `add(a, b)` that returns the sum of two numbers.",
    examples: "-- add(2, 3) -> 5\n-- add(-1, 1) -> 0",
    starterCode: `function add(a, b)
  -- return the sum
end`,
    testCases: [
      { id: 1, input: "add(2, 3)", expectedOutput: "5", setupCode: "print(add(2, 3))" },
      { id: 2, input: "add(-1, 1)", expectedOutput: "0", setupCode: "print(add(-1, 1))" },
      { id: 3, input: "add(100, 250)", expectedOutput: "350", setupCode: "print(add(100, 250))" },
    ],
  },
  {
    id: "is-even",
    title: "Even or Odd",
    difficulty: "easy",
    description: "Write a function `isEven(n)` that returns `true` if the number is even, `false` otherwise.",
    examples: "-- isEven(4) -> true\n-- isEven(7) -> false",
    starterCode: `function isEven(n)
  -- return true or false
end`,
    testCases: [
      { id: 1, input: "isEven(4)", expectedOutput: "true", setupCode: "print(tostring(isEven(4)))" },
      { id: 2, input: "isEven(7)", expectedOutput: "false", setupCode: "print(tostring(isEven(7)))" },
      { id: 3, input: "isEven(0)", expectedOutput: "true", setupCode: "print(tostring(isEven(0)))" },
    ],
  },
  {
    id: "max-of-three",
    title: "Maximum of Three",
    difficulty: "easy",
    description: "Write a function `maxOfThree(a, b, c)` that returns the largest of three numbers.",
    examples: "-- maxOfThree(1, 5, 3) -> 5\n-- maxOfThree(-2, -8, -1) -> -1",
    starterCode: `function maxOfThree(a, b, c)
  -- return the largest number
end`,
    testCases: [
      { id: 1, input: "maxOfThree(1, 5, 3)", expectedOutput: "5", setupCode: "print(maxOfThree(1, 5, 3))" },
      { id: 2, input: "maxOfThree(-2, -8, -1)", expectedOutput: "-1", setupCode: "print(maxOfThree(-2, -8, -1))" },
      { id: 3, input: "maxOfThree(10, 10, 10)", expectedOutput: "10", setupCode: "print(maxOfThree(10, 10, 10))" },
    ],
  },
  {
    id: "absolute-value",
    title: "Absolute Value",
    difficulty: "easy",
    description: "Write a function `absolute(n)` that returns the absolute value of a number (without using `math.abs`).",
    examples: "-- absolute(-5) -> 5\n-- absolute(3) -> 3\n-- absolute(0) -> 0",
    starterCode: `function absolute(n)
  -- return the absolute value
end`,
    testCases: [
      { id: 1, input: "absolute(-5)", expectedOutput: "5", setupCode: "print(absolute(-5))" },
      { id: 2, input: "absolute(3)", expectedOutput: "3", setupCode: "print(absolute(3))" },
      { id: 3, input: "absolute(0)", expectedOutput: "0", setupCode: "print(absolute(0))" },
    ],
  },
  {
    id: "string-length",
    title: "String Length",
    difficulty: "easy",
    description: "Write a function `strLen(s)` that returns the length of a string using the `#` operator.",
    examples: '-- strLen("hello") -> 5\n-- strLen("") -> 0',
    starterCode: `function strLen(s)
  -- return the length
end`,
    testCases: [
      { id: 1, input: 'strLen("hello")', expectedOutput: "5", setupCode: 'print(strLen("hello"))' },
      { id: 2, input: 'strLen("")', expectedOutput: "0", setupCode: 'print(strLen(""))' },
      { id: 3, input: 'strLen("Luau")', expectedOutput: "4", setupCode: 'print(strLen("Luau"))' },
    ],
  },
  {
    id: "multiply",
    title: "Multiply Two Numbers",
    difficulty: "easy",
    description: "Write a function `multiply(a, b)` that returns the product of two numbers.",
    examples: "-- multiply(3, 4) -> 12\n-- multiply(-2, 5) -> -10",
    starterCode: `function multiply(a, b)
  -- return the product
end`,
    testCases: [
      { id: 1, input: "multiply(3, 4)", expectedOutput: "12", setupCode: "print(multiply(3, 4))" },
      { id: 2, input: "multiply(-2, 5)", expectedOutput: "-10", setupCode: "print(multiply(-2, 5))" },
      { id: 3, input: "multiply(0, 100)", expectedOutput: "0", setupCode: "print(multiply(0, 100))" },
    ],
  },
  {
    id: "is-positive",
    title: "Is Positive",
    difficulty: "easy",
    description: "Write a function `isPositive(n)` that returns `true` if n > 0, `false` otherwise.",
    examples: "-- isPositive(5) -> true\n-- isPositive(-3) -> false\n-- isPositive(0) -> false",
    starterCode: `function isPositive(n)
  -- return true or false
end`,
    testCases: [
      { id: 1, input: "isPositive(5)", expectedOutput: "true", setupCode: "print(tostring(isPositive(5)))" },
      { id: 2, input: "isPositive(-3)", expectedOutput: "false", setupCode: "print(tostring(isPositive(-3)))" },
      { id: 3, input: "isPositive(0)", expectedOutput: "false", setupCode: "print(tostring(isPositive(0)))" },
    ],
  },
  {
    id: "last-element",
    title: "Last Element",
    difficulty: "easy",
    description: "Write a function `lastElement(t)` that returns the last element of a table (array).",
    examples: "-- lastElement({1, 2, 3}) -> 3\n-- lastElement({10}) -> 10",
    starterCode: `function lastElement(t)
  -- return the last element
end`,
    testCases: [
      { id: 1, input: "lastElement({1, 2, 3})", expectedOutput: "3", setupCode: "print(lastElement({1, 2, 3}))" },
      { id: 2, input: "lastElement({10})", expectedOutput: "10", setupCode: "print(lastElement({10}))" },
      { id: 3, input: "lastElement({5, 8, 2, 7})", expectedOutput: "7", setupCode: "print(lastElement({5, 8, 2, 7}))" },
    ],
  },
  {
    id: "greet",
    title: "Greet by Name",
    difficulty: "easy",
    description: 'Write a function `greet(name)` that returns the string "Hello, " followed by the name and "!".',
    examples: '-- greet("Alice") -> "Hello, Alice!"',
    starterCode: `function greet(name)
  -- return greeting string
end`,
    testCases: [
      { id: 1, input: 'greet("Alice")', expectedOutput: "Hello, Alice!", setupCode: 'print(greet("Alice"))' },
      { id: 2, input: 'greet("World")', expectedOutput: "Hello, World!", setupCode: 'print(greet("World"))' },
      { id: 3, input: 'greet("Luau")', expectedOutput: "Hello, Luau!", setupCode: 'print(greet("Luau"))' },
    ],
  },
  {
    id: "celsius-to-fahrenheit",
    title: "Celsius to Fahrenheit",
    difficulty: "easy",
    description: "Write a function `toFahrenheit(c)` that converts Celsius to Fahrenheit.\nFormula: F = C * 9/5 + 32",
    examples: "-- toFahrenheit(0) -> 32\n-- toFahrenheit(100) -> 212",
    starterCode: `function toFahrenheit(c)
  -- convert and return
end`,
    testCases: [
      { id: 1, input: "toFahrenheit(0)", expectedOutput: "32", setupCode: "print(toFahrenheit(0))" },
      { id: 2, input: "toFahrenheit(100)", expectedOutput: "212", setupCode: "print(toFahrenheit(100))" },
      { id: 3, input: "toFahrenheit(37)", expectedOutput: "98.6", setupCode: "print(toFahrenheit(37))" },
    ],
  },
  {
    id: "is-adult",
    title: "Age Check",
    difficulty: "easy",
    description: 'Write a function `isAdult(age)` that returns "adult" if age >= 18, "minor" otherwise.',
    examples: '-- isAdult(21) -> "adult"\n-- isAdult(15) -> "minor"',
    starterCode: `function isAdult(age)
  -- return "adult" or "minor"
end`,
    testCases: [
      { id: 1, input: "isAdult(21)", expectedOutput: "adult", setupCode: "print(isAdult(21))" },
      { id: 2, input: "isAdult(15)", expectedOutput: "minor", setupCode: "print(isAdult(15))" },
      { id: 3, input: "isAdult(18)", expectedOutput: "adult", setupCode: "print(isAdult(18))" },
    ],
  },
  {
    id: "countdown",
    title: "Countdown",
    difficulty: "easy",
    description: "Write a function `countdown(n)` that prints numbers from n down to 1, one per line.",
    examples: "-- countdown(3) prints:\n-- 3\n-- 2\n-- 1",
    starterCode: `function countdown(n)
  -- print n down to 1
end`,
    testCases: [
      { id: 1, input: "countdown(3)", expectedOutput: "3\n2\n1", setupCode: "countdown(3)" },
      { id: 2, input: "countdown(5)", expectedOutput: "5\n4\n3\n2\n1", setupCode: "countdown(5)" },
      { id: 3, input: "countdown(1)", expectedOutput: "1", setupCode: "countdown(1)" },
    ],
  },
  {
    id: "min-of-two",
    title: "Minimum of Two",
    difficulty: "easy",
    description: "Write a function `minOfTwo(a, b)` that returns the smaller of two numbers.",
    examples: "-- minOfTwo(3, 7) -> 3\n-- minOfTwo(-1, -5) -> -5",
    starterCode: `function minOfTwo(a, b)
  -- return the smaller number
end`,
    testCases: [
      { id: 1, input: "minOfTwo(3, 7)", expectedOutput: "3", setupCode: "print(minOfTwo(3, 7))" },
      { id: 2, input: "minOfTwo(-1, -5)", expectedOutput: "-5", setupCode: "print(minOfTwo(-1, -5))" },
      { id: 3, input: "minOfTwo(4, 4)", expectedOutput: "4", setupCode: "print(minOfTwo(4, 4))" },
    ],
  },

  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  MEDIUM  (15 challenges, including doubleEach)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  {
    id: "fizzbuzz",
    title: "FizzBuzz",
    difficulty: "medium",
    description:
      'Write a function `fizzbuzz(n)` that prints numbers from 1 to n.\nFor multiples of 3 print "Fizz", for multiples of 5 print "Buzz", and for multiples of both print "FizzBuzz".',
    examples: "-- fizzbuzz(5) prints:\n-- 1\n-- 2\n-- Fizz\n-- 4\n-- Buzz",
    starterCode: `function fizzbuzz(n)
  for i = 1, n do
    -- your logic here
  end
end`,
    testCases: [
      { id: 1, input: "fizzbuzz(5)", expectedOutput: "1\n2\nFizz\n4\nBuzz", setupCode: "fizzbuzz(5)" },
      { id: 2, input: "fizzbuzz(15)", expectedOutput: "1\n2\nFizz\n4\nBuzz\nFizz\n7\n8\nFizz\nBuzz\n11\nFizz\n13\n14\nFizzBuzz", setupCode: "fizzbuzz(15)" },
      { id: 3, input: "fizzbuzz(3)", expectedOutput: "1\n2\nFizz", setupCode: "fizzbuzz(3)" },
    ],
  },
  {
    id: "reverse-string",
    title: "Reverse a String",
    difficulty: "medium",
    description: "Write a function `reverseStr(s)` that returns the reversed version of the input string.",
    examples: '-- reverseStr("hello") -> "olleh"\n-- reverseStr("abc") -> "cba"',
    starterCode: `function reverseStr(s)
  -- return the reversed string
end`,
    testCases: [
      { id: 1, input: 'reverseStr("hello")', expectedOutput: "olleh", setupCode: 'print(reverseStr("hello"))' },
      { id: 2, input: 'reverseStr("abc")', expectedOutput: "cba", setupCode: 'print(reverseStr("abc"))' },
      { id: 3, input: 'reverseStr("a")', expectedOutput: "a", setupCode: 'print(reverseStr("a"))' },
    ],
  },
  {
    id: "count-vowels",
    title: "Count Vowels",
    difficulty: "medium",
    description: "Write a function `countVowels(s)` that returns the number of vowels (a, e, i, o, u) in the string. Case-insensitive.",
    examples: '-- countVowels("hello") -> 2\n-- countVowels("AEIOU") -> 5',
    starterCode: `function countVowels(s)
  local count = 0
  -- your logic here
  return count
end`,
    testCases: [
      { id: 1, input: 'countVowels("hello")', expectedOutput: "2", setupCode: 'print(countVowels("hello"))' },
      { id: 2, input: 'countVowels("AEIOU")', expectedOutput: "5", setupCode: 'print(countVowels("AEIOU"))' },
      { id: 3, input: 'countVowels("xyz")', expectedOutput: "0", setupCode: 'print(countVowels("xyz"))' },
    ],
  },
  {
    id: "sum-table",
    title: "Sum of Table",
    difficulty: "medium",
    description: "Write a function `sumTable(t)` that returns the sum of all numeric values in a table (array).",
    examples: "-- sumTable({1, 2, 3}) -> 6\n-- sumTable({10, -5, 3}) -> 8",
    starterCode: `function sumTable(t)
  local total = 0
  -- iterate and sum
  return total
end`,
    testCases: [
      { id: 1, input: "sumTable({1, 2, 3})", expectedOutput: "6", setupCode: "print(sumTable({1, 2, 3}))" },
      { id: 2, input: "sumTable({10, -5, 3})", expectedOutput: "8", setupCode: "print(sumTable({10, -5, 3}))" },
      { id: 3, input: "sumTable({})", expectedOutput: "0", setupCode: "print(sumTable({}))" },
    ],
  },
  {
    id: "factorial",
    title: "Factorial",
    difficulty: "medium",
    description: "Write a function `factorial(n)` that returns n! (n factorial).\n0! = 1, 1! = 1, 5! = 120.",
    examples: "-- factorial(5) -> 120\n-- factorial(0) -> 1",
    starterCode: `function factorial(n)
  -- return n!
end`,
    testCases: [
      { id: 1, input: "factorial(5)", expectedOutput: "120", setupCode: "print(factorial(5))" },
      { id: 2, input: "factorial(0)", expectedOutput: "1", setupCode: "print(factorial(0))" },
      { id: 3, input: "factorial(7)", expectedOutput: "5040", setupCode: "print(factorial(7))" },
    ],
  },
  {
    id: "power",
    title: "Power Function",
    difficulty: "medium",
    description: "Write a function `power(base, exp)` that returns base raised to the power of exp (without using `math.pow` or `^`). Assume exp >= 0.",
    examples: "-- power(2, 3) -> 8\n-- power(5, 0) -> 1",
    starterCode: `function power(base, exp)
  -- calculate base^exp manually
end`,
    testCases: [
      { id: 1, input: "power(2, 3)", expectedOutput: "8", setupCode: "print(power(2, 3))" },
      { id: 2, input: "power(5, 0)", expectedOutput: "1", setupCode: "print(power(5, 0))" },
      { id: 3, input: "power(3, 4)", expectedOutput: "81", setupCode: "print(power(3, 4))" },
    ],
  },
  {
    id: "find-min",
    title: "Find Minimum in Table",
    difficulty: "medium",
    description: "Write a function `findMin(t)` that returns the smallest number in a table (array). Assume the table has at least one element.",
    examples: "-- findMin({3, 1, 4, 1, 5}) -> 1\n-- findMin({-2, -8, -1}) -> -8",
    starterCode: `function findMin(t)
  -- find and return the minimum
end`,
    testCases: [
      { id: 1, input: "findMin({3, 1, 4, 1, 5})", expectedOutput: "1", setupCode: "print(findMin({3, 1, 4, 1, 5}))" },
      { id: 2, input: "findMin({-2, -8, -1})", expectedOutput: "-8", setupCode: "print(findMin({-2, -8, -1}))" },
      { id: 3, input: "findMin({42})", expectedOutput: "42", setupCode: "print(findMin({42}))" },
    ],
  },
  {
    id: "count-char",
    title: "Count Character",
    difficulty: "medium",
    description: "Write a function `countChar(s, ch)` that returns how many times the character `ch` appears in string `s`.",
    examples: '-- countChar("banana", "a") -> 3\n-- countChar("hello", "z") -> 0',
    starterCode: `function countChar(s, ch)
  local count = 0
  -- your logic here
  return count
end`,
    testCases: [
      { id: 1, input: 'countChar("banana", "a")', expectedOutput: "3", setupCode: 'print(countChar("banana", "a"))' },
      { id: 2, input: 'countChar("hello", "z")', expectedOutput: "0", setupCode: 'print(countChar("hello", "z"))' },
      { id: 3, input: 'countChar("aaa", "a")', expectedOutput: "3", setupCode: 'print(countChar("aaa", "a"))' },
    ],
  },
  {
    id: "is-prime",
    title: "Is Prime",
    difficulty: "medium",
    description: "Write a function `isPrime(n)` that returns `true` if n is a prime number, `false` otherwise. Assume n >= 1.",
    examples: "-- isPrime(7) -> true\n-- isPrime(4) -> false\n-- isPrime(1) -> false",
    starterCode: `function isPrime(n)
  -- check if n is prime
end`,
    testCases: [
      { id: 1, input: "isPrime(7)", expectedOutput: "true", setupCode: "print(tostring(isPrime(7)))" },
      { id: 2, input: "isPrime(4)", expectedOutput: "false", setupCode: "print(tostring(isPrime(4)))" },
      { id: 3, input: "isPrime(1)", expectedOutput: "false", setupCode: "print(tostring(isPrime(1)))" },
    ],
  },
  {
    id: "title-case",
    title: "Title Case",
    difficulty: "medium",
    description: 'Write a function `titleCase(s)` that capitalizes the first letter of each word in a string. Words are separated by spaces.',
    examples: '-- titleCase("hello world") -> "Hello World"\n-- titleCase("luau is fun") -> "Luau Is Fun"',
    starterCode: `function titleCase(s)
  -- capitalize first letter of each word
end`,
    testCases: [
      { id: 1, input: 'titleCase("hello world")', expectedOutput: "Hello World", setupCode: 'print(titleCase("hello world"))' },
      { id: 2, input: 'titleCase("luau is fun")', expectedOutput: "Luau Is Fun", setupCode: 'print(titleCase("luau is fun"))' },
      { id: 3, input: 'titleCase("a")', expectedOutput: "A", setupCode: 'print(titleCase("a"))' },
    ],
  },
  {
    id: "repeat-string",
    title: "Repeat String",
    difficulty: "medium",
    description: "Write a function `repeatStr(s, n)` that returns the string `s` repeated `n` times concatenated together.",
    examples: '-- repeatStr("ab", 3) -> "ababab"\n-- repeatStr("x", 5) -> "xxxxx"',
    starterCode: `function repeatStr(s, n)
  -- return s repeated n times
end`,
    testCases: [
      { id: 1, input: 'repeatStr("ab", 3)', expectedOutput: "ababab", setupCode: 'print(repeatStr("ab", 3))' },
      { id: 2, input: 'repeatStr("x", 5)', expectedOutput: "xxxxx", setupCode: 'print(repeatStr("x", 5))' },
      { id: 3, input: 'repeatStr("hi", 1)', expectedOutput: "hi", setupCode: 'print(repeatStr("hi", 1))' },
    ],
  },
  {
    id: "find-index",
    title: "Find Index",
    difficulty: "medium",
    description: "Write a function `findIndex(t, value)` that returns the index of `value` in the table, or -1 if not found. Tables are 1-indexed in Luau.",
    examples: "-- findIndex({10, 20, 30}, 20) -> 2\n-- findIndex({1, 2, 3}, 5) -> -1",
    starterCode: `function findIndex(t, value)
  -- return index or -1
end`,
    testCases: [
      { id: 1, input: "findIndex({10, 20, 30}, 20)", expectedOutput: "2", setupCode: "print(findIndex({10, 20, 30}, 20))" },
      { id: 2, input: "findIndex({1, 2, 3}, 5)", expectedOutput: "-1", setupCode: "print(findIndex({1, 2, 3}, 5))" },
      { id: 3, input: "findIndex({5, 5, 5}, 5)", expectedOutput: "1", setupCode: "print(findIndex({5, 5, 5}, 5))" },
    ],
  },
  {
    id: "join-strings",
    title: "Join Table",
    difficulty: "medium",
    description: 'Write a function `joinTable(t, sep)` that joins all elements of a string table with the separator `sep`.',
    examples: '-- joinTable({"a", "b", "c"}, "-") -> "a-b-c"\n-- joinTable({"hello", "world"}, " ") -> "hello world"',
    starterCode: `function joinTable(t, sep)
  -- join elements with separator
end`,
    testCases: [
      { id: 1, input: 'joinTable({"a","b","c"}, "-")', expectedOutput: "a-b-c", setupCode: 'print(joinTable({"a","b","c"}, "-"))' },
      { id: 2, input: 'joinTable({"hello","world"}, " ")', expectedOutput: "hello world", setupCode: 'print(joinTable({"hello","world"}, " "))' },
      { id: 3, input: 'joinTable({"x"}, ",")', expectedOutput: "x", setupCode: 'print(joinTable({"x"}, ","))' },
    ],
  },
  {
    id: "double-each",
    title: "Double Each Element",
    difficulty: "medium",
    description: "Write a function `doubleEach(t)` that prints each element of the table multiplied by 2, one per line.",
    examples: "-- doubleEach({1, 2, 3}) prints:\n-- 2\n-- 4\n-- 6",
    starterCode: `function doubleEach(t)
  -- print each element * 2
end`,
    testCases: [
      { id: 1, input: "doubleEach({1, 2, 3})", expectedOutput: "2\n4\n6", setupCode: "doubleEach({1, 2, 3})" },
      { id: 2, input: "doubleEach({0, -5, 10})", expectedOutput: "0\n-10\n20", setupCode: "doubleEach({0, -5, 10})" },
      { id: 3, input: "doubleEach({7})", expectedOutput: "14", setupCode: "doubleEach({7})" },
    ],
  },

  {
    id: "sum-digits",
    title: "Sum of Digits",
    difficulty: "medium",
    description: "Write a function `sumDigits(n)` that returns the sum of all digits of a positive integer.",
    examples: "-- sumDigits(123) -> 6   (1+2+3)\n-- sumDigits(9) -> 9",
    starterCode: `function sumDigits(n)
  -- sum each digit of n
end`,
    testCases: [
      { id: 1, input: "sumDigits(123)", expectedOutput: "6", setupCode: "print(sumDigits(123))" },
      { id: 2, input: "sumDigits(9)", expectedOutput: "9", setupCode: "print(sumDigits(9))" },
      { id: 3, input: "sumDigits(9999)", expectedOutput: "36", setupCode: "print(sumDigits(9999))" },
    ],
  },

  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  //  HARD  (11 challenges)
  // ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

  {
    id: "fibonacci",
    title: "Fibonacci Sequence",
    difficulty: "hard",
    description: "Write a function `fibonacci(n)` that prints the first `n` numbers of the Fibonacci sequence, one per line. The sequence starts with 0, 1, 1, 2, 3, 5, ...",
    examples: "-- fibonacci(5) prints:\n-- 0\n-- 1\n-- 1\n-- 2\n-- 3",
    starterCode: `function fibonacci(n)
  -- print first n fibonacci numbers
end`,
    testCases: [
      { id: 1, input: "fibonacci(5)", expectedOutput: "0\n1\n1\n2\n3", setupCode: "fibonacci(5)" },
      { id: 2, input: "fibonacci(1)", expectedOutput: "0", setupCode: "fibonacci(1)" },
      { id: 3, input: "fibonacci(8)", expectedOutput: "0\n1\n1\n2\n3\n5\n8\n13", setupCode: "fibonacci(8)" },
    ],
  },
  {
    id: "palindrome",
    title: "Palindrome Check",
    difficulty: "hard",
    description: 'Write a function `isPalindrome(s)` that returns `true` if the string reads the same forwards and backwards (case-insensitive), `false` otherwise.',
    examples: '-- isPalindrome("racecar") -> true\n-- isPalindrome("hello") -> false\n-- isPalindrome("Aba") -> true',
    starterCode: `function isPalindrome(s)
  -- check if s is a palindrome
end`,
    testCases: [
      { id: 1, input: 'isPalindrome("racecar")', expectedOutput: "true", setupCode: 'print(tostring(isPalindrome("racecar")))' },
      { id: 2, input: 'isPalindrome("hello")', expectedOutput: "false", setupCode: 'print(tostring(isPalindrome("hello")))' },
      { id: 3, input: 'isPalindrome("Aba")', expectedOutput: "true", setupCode: 'print(tostring(isPalindrome("Aba")))' },
    ],
  },
  {
    id: "gcd",
    title: "Greatest Common Divisor",
    difficulty: "hard",
    description: "Write a function `gcd(a, b)` that returns the greatest common divisor of two positive integers using the Euclidean algorithm.",
    examples: "-- gcd(12, 8) -> 4\n-- gcd(17, 5) -> 1\n-- gcd(100, 75) -> 25",
    starterCode: `function gcd(a, b)
  -- Euclidean algorithm
end`,
    testCases: [
      { id: 1, input: "gcd(12, 8)", expectedOutput: "4", setupCode: "print(gcd(12, 8))" },
      { id: 2, input: "gcd(17, 5)", expectedOutput: "1", setupCode: "print(gcd(17, 5))" },
      { id: 3, input: "gcd(100, 75)", expectedOutput: "25", setupCode: "print(gcd(100, 75))" },
    ],
  },
  {
    id: "bubble-sort",
    title: "Bubble Sort",
    difficulty: "hard",
    description: "Write a function `bubbleSort(t)` that sorts a table of numbers in ascending order using bubble sort, then prints each element one per line.",
    examples: "-- bubbleSort({3, 1, 2}) prints:\n-- 1\n-- 2\n-- 3",
    starterCode: `function bubbleSort(t)
  -- sort the table, then print each element
end`,
    testCases: [
      { id: 1, input: "bubbleSort({3, 1, 2})", expectedOutput: "1\n2\n3", setupCode: "bubbleSort({3, 1, 2})" },
      { id: 2, input: "bubbleSort({5, -1, 4, 0})", expectedOutput: "-1\n0\n4\n5", setupCode: "bubbleSort({5, -1, 4, 0})" },
      { id: 3, input: "bubbleSort({1})", expectedOutput: "1", setupCode: "bubbleSort({1})" },
    ],
  },
  {
    id: "caesar-cipher",
    title: "Caesar Cipher",
    difficulty: "hard",
    description: "Write a function `caesarEncrypt(s, shift)` that shifts each lowercase letter by `shift` positions in the alphabet, wrapping around. Non-lowercase characters remain unchanged.",
    examples: '-- caesarEncrypt("abc", 1) -> "bcd"\n-- caesarEncrypt("xyz", 3) -> "abc"',
    starterCode: `function caesarEncrypt(s, shift)
  -- shift each lowercase letter
end`,
    testCases: [
      { id: 1, input: 'caesarEncrypt("abc", 1)', expectedOutput: "bcd", setupCode: 'print(caesarEncrypt("abc", 1))' },
      { id: 2, input: 'caesarEncrypt("xyz", 3)', expectedOutput: "abc", setupCode: 'print(caesarEncrypt("xyz", 3))' },
      { id: 3, input: 'caesarEncrypt("hello", 13)', expectedOutput: "uryyb", setupCode: 'print(caesarEncrypt("hello", 13))' },
    ],
  },
  {
    id: "two-sum",
    title: "Two Sum",
    difficulty: "hard",
    description: "Write a function `twoSum(t, target)` that finds two numbers in the table that add up to `target` and prints their indices (1-based), separated by a space. If no pair exists, print -1. Return the first valid pair found (smallest first index).",
    examples: '-- twoSum({2, 7, 11, 15}, 9) -> "1 2"\n-- twoSum({1, 2, 3}, 10) -> "-1"',
    starterCode: `function twoSum(t, target)
  -- find indices of two numbers that sum to target
end`,
    testCases: [
      { id: 1, input: "twoSum({2, 7, 11, 15}, 9)", expectedOutput: "1 2", setupCode: "twoSum({2, 7, 11, 15}, 9)" },
      { id: 2, input: "twoSum({1, 2, 3}, 10)", expectedOutput: "-1", setupCode: "twoSum({1, 2, 3}, 10)" },
      { id: 3, input: "twoSum({3, 5, 1, 4}, 9)", expectedOutput: "2 4", setupCode: "twoSum({3, 5, 1, 4}, 9)" },
    ],
  },
  {
    id: "flatten-table",
    title: "Flatten Nested Table",
    difficulty: "hard",
    description: "Write a function `flatten(t)` that takes a nested table (up to 2 levels deep) and prints all values in order, one per line. Sub-tables should be expanded inline.",
    examples: "-- flatten({1, {2, 3}, 4}) prints:\n-- 1\n-- 2\n-- 3\n-- 4",
    starterCode: `function flatten(t)
  -- print all values, expanding sub-tables
end`,
    testCases: [
      { id: 1, input: "flatten({1, {2, 3}, 4})", expectedOutput: "1\n2\n3\n4", setupCode: "flatten({1, {2, 3}, 4})" },
      { id: 2, input: "flatten({{1, 2}, {3, 4}})", expectedOutput: "1\n2\n3\n4", setupCode: "flatten({{1, 2}, {3, 4}})" },
      { id: 3, input: "flatten({5})", expectedOutput: "5", setupCode: "flatten({5})" },
    ],
  },
  {
    id: "anagram-check",
    title: "Anagram Check",
    difficulty: "hard",
    description: 'Write a function `isAnagram(a, b)` that returns `true` if strings `a` and `b` are anagrams of each other (same characters, same count, case-insensitive), `false` otherwise.',
    examples: '-- isAnagram("listen", "silent") -> true\n-- isAnagram("hello", "world") -> false',
    starterCode: `function isAnagram(a, b)
  -- check if a and b are anagrams
end`,
    testCases: [
      { id: 1, input: 'isAnagram("listen", "silent")', expectedOutput: "true", setupCode: 'print(tostring(isAnagram("listen", "silent")))' },
      { id: 2, input: 'isAnagram("hello", "world")', expectedOutput: "false", setupCode: 'print(tostring(isAnagram("hello", "world")))' },
      { id: 3, input: 'isAnagram("Astronomer", "Moon starer")', expectedOutput: "false", setupCode: 'print(tostring(isAnagram("Astronomer", "Moon starer")))' },
    ],
  },
  {
    id: "run-length-encoding",
    title: "Run-Length Encoding",
    difficulty: "hard",
    description: 'Write a function `rle(s)` that compresses a string using run-length encoding. Consecutive identical characters are replaced by the character followed by its count.',
    examples: '-- rle("aaabbc") -> "a3b2c1"\n-- rle("aabba") -> "a2b2a1"',
    starterCode: `function rle(s)
  -- return run-length encoded string
end`,
    testCases: [
      { id: 1, input: 'rle("aaabbc")', expectedOutput: "a3b2c1", setupCode: 'print(rle("aaabbc"))' },
      { id: 2, input: 'rle("aabba")', expectedOutput: "a2b2a1", setupCode: 'print(rle("aabba"))' },
      { id: 3, input: 'rle("x")', expectedOutput: "x1", setupCode: 'print(rle("x"))' },
    ],
  },
  {
    id: "matrix-sum",
    title: "Matrix Diagonal Sum",
    difficulty: "hard",
    description: "Write a function `diagSum(m)` that returns the sum of the main diagonal of a square matrix (table of tables). The main diagonal consists of elements where row index equals column index.",
    examples: "-- diagSum({{1,2,3},{4,5,6},{7,8,9}}) -> 15  (1+5+9)",
    starterCode: `function diagSum(m)
  -- sum the main diagonal
end`,
    testCases: [
      { id: 1, input: "diagSum({{1,2,3},{4,5,6},{7,8,9}})", expectedOutput: "15", setupCode: "print(diagSum({{1,2,3},{4,5,6},{7,8,9}}))" },
      { id: 2, input: "diagSum({{5}})", expectedOutput: "5", setupCode: "print(diagSum({{5}}))" },
      { id: 3, input: "diagSum({{1,0},{0,1}})", expectedOutput: "2", setupCode: "print(diagSum({{1,0},{0,1}}))" },
    ],
  },
  {
    id: "merge-sorted",
    title: "Merge Sorted Tables",
    difficulty: "hard",
    description: "Write a function `mergeSorted(a, b)` that merges two already-sorted tables into a single sorted table and prints each element one per line.",
    examples: "-- mergeSorted({1, 3, 5}, {2, 4, 6}) prints:\n-- 1\n-- 2\n-- 3\n-- 4\n-- 5\n-- 6",
    starterCode: `function mergeSorted(a, b)
  -- merge two sorted tables and print
end`,
    testCases: [
      { id: 1, input: "mergeSorted({1,3,5},{2,4,6})", expectedOutput: "1\n2\n3\n4\n5\n6", setupCode: "mergeSorted({1,3,5},{2,4,6})" },
      { id: 2, input: "mergeSorted({1,2},{3,4,5})", expectedOutput: "1\n2\n3\n4\n5", setupCode: "mergeSorted({1,2},{3,4,5})" },
      { id: 3, input: "mergeSorted({},{1,2})", expectedOutput: "1\n2", setupCode: "mergeSorted({},{1,2})" },
    ],
  },
];
