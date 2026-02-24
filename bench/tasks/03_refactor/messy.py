def f(x):
    r=[]
    for i in range(1,x+1):
        if i%15==0:r.append("FizzBuzz")
        elif i%3==0:r.append("Fizz")
        elif i%5==0:r.append("Buzz")
        else:r.append(str(i))
    return r
