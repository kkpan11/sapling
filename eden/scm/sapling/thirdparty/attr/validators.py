"""
Commonly useful validators.
"""



from ._make import _AndValidator, and_, attr, attributes


__all__ = ["and_", "in_", "instance_of", "optional", "provides"]


@attributes(repr=False, slots=True, hash=True)
class _InstanceOfValidator:
    type = attr()

    def __call__(self, inst, attr, value):
        """
        We use a callable class to be able to change the ``__repr__``.
        """
        if not isinstance(value, self.type):
            raise TypeError(
                "'{name}' must be {type!r} (got {value!r} that is a "
                "{actual!r}).".format(
                    name=attr.name, type=self.type, actual=value.__class__, value=value
                ),
                attr,
                self.type,
                value,
            )

    def __repr__(self):
        return "<instance_of validator for type {type!r}>".format(type=self.type)


def instance_of(type) -> _InstanceOfValidator:
    """
    A validator that raises a :exc:`TypeError` if the initializer is called
    with a wrong type for this particular attribute (checks are performed using
    :func:`isinstance` therefore it's also valid to pass a tuple of types).

    :param type: The type to check for.
    :type type: type or tuple of types

    :raises TypeError: With a human readable error message, the attribute
        (of type :class:`attr.Attribute`), the expected type, and the value it
        got.
    """
    # pyre-fixme[19]: Expected 0 positional arguments.
    return _InstanceOfValidator(type)


@attributes(repr=False, slots=True, hash=True)
class _ProvidesValidator:
    interface = attr()

    def __call__(self, inst, attr, value):
        """
        We use a callable class to be able to change the ``__repr__``.
        """
        if not self.interface.providedBy(value):
            raise TypeError(
                "'{name}' must provide {interface!r} which {value!r} "
                "doesn't.".format(
                    name=attr.name, interface=self.interface, value=value
                ),
                attr,
                self.interface,
                value,
            )

    def __repr__(self):
        return "<provides validator for interface {interface!r}>".format(
            interface=self.interface
        )


def provides(interface) -> _ProvidesValidator:
    """
    A validator that raises a :exc:`TypeError` if the initializer is called
    with an object that does not provide the requested *interface* (checks are
    performed using ``interface.providedBy(value)`` (see `zope.interface
    <https://zopeinterface.readthedocs.io/en/latest/>`_).

    :param zope.interface.Interface interface: The interface to check for.

    :raises TypeError: With a human readable error message, the attribute
        (of type :class:`attr.Attribute`), the expected interface, and the
        value it got.
    """
    # pyre-fixme[19]: Expected 0 positional arguments.
    return _ProvidesValidator(interface)


@attributes(repr=False, slots=True, hash=True)
class _OptionalValidator:
    validator = attr()

    def __call__(self, inst, attr, value):
        if value is None:
            return

        self.validator(inst, attr, value)

    def __repr__(self):
        return "<optional validator for {what} or None>".format(
            what=repr(self.validator)
        )


def optional(validator) -> _OptionalValidator:
    """
    A validator that makes an attribute optional.  An optional attribute is one
    which can be set to ``None`` in addition to satisfying the requirements of
    the sub-validator.

    :param validator: A validator (or a list of validators) that is used for
        non-``None`` values.
    :type validator: callable or :class:`list` of callables.

    .. versionadded:: 15.1.0
    .. versionchanged:: 17.1.0 *validator* can be a list of validators.
    """
    if isinstance(validator, list):
        # pyre-fixme[19]: Expected 0 positional arguments.
        return _OptionalValidator(_AndValidator(validator))
    # pyre-fixme[19]: Expected 0 positional arguments.
    return _OptionalValidator(validator)


@attributes(repr=False, slots=True, hash=True)
class _InValidator:
    options = attr()

    def __call__(self, inst, attr, value):
        if value not in self.options:
            raise ValueError(
                "'{name}' must be in {options!r} (got {value!r})".format(
                    name=attr.name, options=self.options, value=value
                )
            )

    def __repr__(self):
        return "<in_ validator with options {options!r}>".format(options=self.options)


def in_(options) -> _InValidator:
    """
    A validator that raises a :exc:`ValueError` if the initializer is called
    with a value that does not belong in the options provided.  The check is
    performed using ``value in options``.

    :param options: Allowed options.
    :type options: list, tuple, :class:`enum.Enum`, ...

    :raises ValueError: With a human readable error message, the attribute (of
       type :class:`attr.Attribute`), the expected options, and the value it
       got.

    .. versionadded:: 17.1.0
    """
    # pyre-fixme[19]: Expected 0 positional arguments.
    return _InValidator(options)
